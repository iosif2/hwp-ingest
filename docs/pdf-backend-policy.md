# PDF backend policy

## Status

Accepted.

## Decision

`hwp-ingest`의 기본 PDF backend는 다음 정책을 따른다.

```text
default PDF backend:
  - if rsvg-convert exists: use rsvg-convert
  - else: return clear error explaining that current svg2pdf backend is known-broken for clipped table text
```

즉 기본 경로에서 `svg2pdf`로 조용히 fallback하지 않는다.

## Context

현재 vendored `rhwp`의 PDF path는 SVG render output을 `svg2pdf`로 변환한다. 실제 문서 변환 검증에서 표 셀 안의 텍스트가 SVG에는 존재하지만, `svg2pdf` 변환 결과 PDF에서는 시각적으로 누락되는 문제가 확인되었다.

같은 SVG를 `rsvg-convert`로 PDF 변환하면 표 셀 텍스트가 표시된다. 따라서 문제 위치는 HWP parsing, layout, SVG rendering, Python binding, engine API가 아니라 `svg2pdf` 기반 PDF backend다.

## Scope

이 결정은 PDF rendering backend 선택에만 적용된다.

- `vendor/rhwp-subset`은 upstream-derived parser/layout/render source를 보존한다.
- `hwp-ingest-rhwp-adapter`는 `rhwp`가 만든 SVG page output을 받아 PDF backend를 선택한다.
- `hwp-ingest` engine은 PDF backend 구현 세부사항을 직접 알지 않는다.
- Python facade는 backend별 변환 loop나 fallback policy를 소유하지 않는다.

## Runtime policy

기본 PDF 변환은 adapter 안에서 다음 순서로 처리한다.

1. `rhwp`로 HWP 문서를 parse/layout/render한다.
2. 변환 대상 page를 SVG page output으로 만든다.
3. runtime에서 `rsvg-convert` 실행 파일을 찾는다.
4. `rsvg-convert`가 있으면 SVG page들을 `rsvg-convert -f pdf`로 변환한다.
5. `rsvg-convert`가 없으면 변환을 실패시킨다.
6. 실패 에러는 현재 `svg2pdf` backend가 clipped table text를 누락하는 known-broken backend라서 silent fallback하지 않는다는 사실을 설명해야 한다.

## Error behavior

`rsvg-convert`가 없는 환경에서 기본 PDF 변환은 성공한 것처럼 PDF를 만들면 안 된다. 에러는 적어도 다음 정보를 포함해야 한다.

- `rsvg-convert` executable이 없다는 사실.
- 현재 `svg2pdf` fallback은 clipped table text 손실이 확인되어 기본 backend로 사용할 수 없다는 사실.
- 사용자가 취할 수 있는 조치: `rsvg-convert`/librsvg tools 설치, 또는 명시적인 experimental backend 선택이 구현된 경우에만 `svg2pdf` 사용.

예시 문구:

```text
rsvg-convert executable was not found. Install librsvg tools to enable the default PDF backend. The svg2pdf backend is not used as an automatic fallback because it is known to drop clipped table text in some HWP documents.
```

## Implementation boundary

권장 구현 위치는 adapter다.

```text
crates/hwp-ingest-rhwp-adapter/
  src/
    render.rs
    pdf_backend.rs
```

권장 내부 구조:

```rust
pub(crate) enum PdfBackend {
    RsvgConvert,
}

pub(crate) fn svgs_to_pdf(svg_pages: &[String], fonts: &PdfFontOptions) -> Result<Vec<u8>, RhwpAdapterError>;
```

`PdfFontOptions`는 font policy 절을 따른다.

`svg2pdf`를 남겨야 한다면 명시적인 experimental/debug backend로만 둔다. 기본 backend selection에서 `svg2pdf`를 자동 fallback으로 사용하지 않는다.

## Process execution rules

`rsvg-convert` 실행은 shell을 통하지 않고 `std::process::Command`로 직접 수행한다.

금지:

```rust
Command::new("sh").arg("-c").arg("rsvg-convert ...")
```

허용:

```rust
Command::new("rsvg-convert")
    .arg("-f")
    .arg("pdf")
    .arg("-o")
    .arg(&output_path)
    .args(svg_paths)
```

파일명에는 공백과 한글이 포함될 수 있으므로 모든 path는 shell string으로 합치지 말고 argument로 전달한다.

## Multi-page behavior

`rsvg-convert`는 여러 SVG input file을 하나의 PDF output으로 변환할 수 있다. Adapter는 page별 SVG를 temporary directory에 순서대로 쓰고, 그 path 목록을 `rsvg-convert`에 전달하는 방식을 기본 구현으로 사용한다.

```text
rsvg-convert -f pdf -o output.pdf page-0001.svg page-0002.svg ...
```

`to_pdf_bytes`는 temporary PDF output을 생성한 뒤 bytes로 읽어 반환한다. `to_pdf_file`은 요청된 output path로 직접 쓰거나 temporary output을 atomic하게 이동한다.

## Font policy

PDF의 글자 모양은 `rsvg-convert`가 실행 환경의 fontconfig로 고른 폰트가 정한다. SVG 렌더러는 문서 폰트 뒤에 한글 대체 체인과 마지막 `serif`/`sans-serif`를 붙이지만, 실행 환경에 한글 폰트가 하나도 없으면 체인 전체가 한글 글리프가 없는 폰트(예: DejaVu)로 귀결되어 한글이 빈 상자(□)로 나온다. 이는 서버나 컨테이너처럼 폰트를 설치하지 않은 환경에서 흔하다.

결정:

- 어댑터는 변환마다 임시 workspace에 fontconfig 설정을 만들어 `rsvg-convert` 프로세스에만 `FONTCONFIG_FILE`로 전달한다. 프로세스 전역 환경은 바꾸지 않는다.
- 설정은 시스템 설정 위에 호출자의 `font_dirs`와 내장 대체 폰트 폴더를 더한다. 시스템에 폰트가 있으면 그 폰트가 먼저 쓰이므로 한글 폰트가 있는 환경의 결과는 바뀌지 않는다.
- 기본 대체 폰트는 Noto Sans KR Regular(SIL OFL 1.1) 서브셋 하나이며 바이너리에 내장한다. 출처, 해시, 글자 범위는 `crates/hwp-ingest-rhwp-adapter/assets/fonts/README.md`에 기록하고 라이선스 전문은 `licenses/NotoSansKR-OFL.txt`에 둔다.
- 호출자의 `FONTCONFIG_FILE`이 있으면 그 파일을 기반으로 삼고, 없으면 `/etc/fonts/fonts.conf` 등 알려진 시스템 위치를 쓴다. 기반 설정 파일을 찾지 못하면 시스템 폰트까지 잃을 수 있으므로 설정을 만들지 않고 기존 동작을 유지한다.
- `<cachedir>`는 `<include>`보다 앞에 둔다. fontconfig는 새 캐시를 첫 번째 쓰기 가능한 캐시 폴더에 쓰는데, 대체 폰트 폴더의 경로가 변환마다 달라서 뒤에 두면 사용자의 `~/.cache/fontconfig`에 오래된 캐시 파일이 계속 쌓인다.
- 설정을 만드는 데 실패하면 변환은 계속하고 대체 폰트만 빠진다.

이 폰트가 해결하는 것은 한글뿐이다. 측정한 범위는 한글 음절 11,172자 100%, 라틴 100%이고, 한자(CJK Unified), 원문자(①), 전각 기호, 대부분의 화살표·수학 기호, 한컴 전용 PUA 기호는 0%에 가깝다. 이 글자들은 시스템 폰트(예: `fonts-noto-cjk`)나 `font_dirs`가 필요하다. 내장 폰트는 고딕 Regular 한 굵기라서 명조 문서도 이 고딕으로 그려지고 굵은 글씨는 굵기가 합성된다(PDF에 Bold 폰트가 따로 임베드되지 않는 것을 `pdffonts`로 확인했다).

SVG 출력은 폰트를 담을 수 없으므로 이 정책의 대상이 아니다. SVG를 보는 쪽에 한글 폰트가 있어야 한다.

macOS·Windows에서는 fontconfig 위치와 동작을 검증하지 않았다. 알려진 시스템 설정 파일이 없으면 대체 설정을 만들지 않으므로 기존 동작이 유지된다.

## Non-goals

이 결정은 다음을 의미하지 않는다.

- `rsvg-convert`를 public Python API 옵션으로 즉시 노출한다.
- `librsvg` Rust binding을 build dependency로 추가한다.
- `svg2pdf` backend를 삭제한다.
- semantic output, SVG export, image export API를 동시에 구현한다.
- PDF fidelity 문제가 모두 해결됐다고 간주한다.
- 한자와 기호를 포함한 전체 CJK 폰트를 내장한다(font policy 참고).
- 한컴·MS 등 재배포 불가 폰트를 내장한다.

## Verification

이 backend policy를 구현할 때는 최소한 다음 검증을 수행한다.

1. `rsvg-convert`가 설치된 환경에서 실제 HWP page를 PDF로 변환한다.
2. 표 셀 텍스트가 SVG와 PDF render 양쪽에서 보존되는지 확인한다.
3. `rsvg-convert`가 없는 환경을 시뮬레이션하여 clear error가 반환되는지 확인한다.
4. 기본 경로가 `svg2pdf`로 silent fallback하지 않는지 확인한다.

사용자 제공 private HWP 문서는 repository fixture로 커밋하지 않는다. 회귀 테스트가 필요하면 공개 가능한 최소 fixture를 별도로 만든다.
5. 한글 폰트가 없는 환경(또는 `FONTCONFIG_FILE`을 폰트가 없는 설정으로 지정한 테스트)에서 변환한 PDF에 내장 대체 폰트가 임베드되는지 확인한다.
6. `font_dirs`로 준 폰트가 내장 폰트보다 우선해서 임베드되는지 확인한다.
7. 변환 후 사용자의 fontconfig 캐시 폴더에 파일이 남지 않는지 확인한다.

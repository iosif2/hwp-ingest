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

pub(crate) fn svgs_to_pdf(svg_pages: &[String]) -> Result<Vec<u8>, RhwpAdapterError>;
```

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

## Non-goals

이 결정은 다음을 의미하지 않는다.

- `rsvg-convert`를 public Python API 옵션으로 즉시 노출한다.
- `librsvg` Rust binding을 build dependency로 추가한다.
- `svg2pdf` backend를 삭제한다.
- semantic output, SVG export, image export API를 동시에 구현한다.
- PDF fidelity 문제가 모두 해결됐다고 간주한다.

## Verification

이 backend policy를 구현할 때는 최소한 다음 검증을 수행한다.

1. `rsvg-convert`가 설치된 환경에서 실제 HWP page를 PDF로 변환한다.
2. 표 셀 텍스트가 SVG와 PDF render 양쪽에서 보존되는지 확인한다.
3. `rsvg-convert`가 없는 환경을 시뮬레이션하여 clear error가 반환되는지 확인한다.
4. 기본 경로가 `svg2pdf`로 silent fallback하지 않는지 확인한다.

사용자 제공 private HWP 문서는 repository fixture로 커밋하지 않는다. 회귀 테스트가 필요하면 공개 가능한 최소 fixture를 별도로 만든다.

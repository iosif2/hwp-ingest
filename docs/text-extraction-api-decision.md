# Text extraction API decision

## Status

Accepted. Implemented in the adapter, engine, and Python facade.

## Context

분류·검색 같은 downstream 소비자는 HWP·HWPX 첨부파일의 일부 페이지(대표적으로 첫 페이지) 텍스트를 선택적으로 필요로 한다. 요구사항은 다음과 같다.

1. 지정한 페이지 범위의 내장 텍스트를 추출한다. 첫 페이지 선택도 이 범위에 포함한다.
2. 텍스트 없음, 추출 실패, 페이지 경계 미지원, 잘못된 범위를 서로 혼동하지 않는다.
3. 페이지를 식별할 수 없으면 문서 앞부분 발췌를 첫 페이지 텍스트로 표시하지 않는다.
4. 추출 실패나 텍스트 없음일 때도 호출 측이 파일명이나 게시글 같은 다른 정보만으로 진행할 수 있게 결과를 전달한다.

범위 밖: OCR, 이미지 렌더링, 객체 저장소 다운로드, 분류 호출, 업무 DB 저장, 수집 정책.

새 파서는 필요하지 않았다. vendored `rhwp`가 이미 렌더와 같은 페이지 트리에서 페이지 텍스트를 만들고, `service::DocumentService`가 열기 실패를 타입으로 구분한다.

## Facts the design rests on

- HWP 5.0과 HWPX는 페이지 경계를 문서 데이터로 저장하지 않는다. 페이지는 layout engine이 조판으로 계산한다. HWP 5.0 `HwpSummaryInformation`의 페이지 수는 신뢰할 수 없고(값이 있는 파일 대부분이 0 또는 1), HWPX 패키지에는 페이지 수 필드가 없다.
- 텍스트 추출은 PDF/SVG 변환과 같은 `build_page_tree`를 쓰므로 페이지 경계가 변환 결과와 같다.
- 한컴 오피스가 출력한 참조 PDF와 비교했을 때, 다쪽 문서의 첫 페이지 텍스트는 대부분 거의 같았다. 쪽 단위로 경계가 어긋난 경우는 측정한 코퍼스에서 약 0.4%였다. 이 코퍼스는 upstream이 해당 PDF에 맞춰 조정해 온 회귀 샘플이므로 현장 문서의 일치율은 더 낮을 수 있다.
- 텍스트가 없는 페이지(이미지·도형 위주)는 흔한 정상 상태다.

## Decision

### Boundaries

- 기존 PDF/SVG 변환 경로와 그 열기 호출은 변경하지 않는다. 텍스트 API만 `rhwp::service::DocumentService`로 문서를 연다. 서비스는 내부에서 같은 `DocumentCore::from_bytes`를 호출하므로 파싱은 같고, 차이는 입력 크기 상한(256MB)과 사전 형식 판별뿐이다.
- `hwp-ingest-rhwp-adapter`가 `ServiceError`와 `FileFormat`을 타입으로 분류한다. 메시지 문자열을 해석하지 않는다.
- `hwp-ingest` engine이 공개 타입, 상태 코드, 출처 정보, 해시를 소유한다. PyO3 layer는 변환만 하고, Python facade는 이름, `range` 변환, PathLike 정규화, 스텁을 소유한다.
- vendored `rhwp`에는 래퍼 하나만 추가한다: `DocumentCore::extract_page_text_native_with_compat`. 기존 `render_page_svg_native_with_compat`와 같은 패턴이며 `vendor/rhwp-subset/LOCAL_CHANGES.md`에 기록한다.

### Page selection

- 페이지는 0-based이고 범위는 반열린 `[start, end)`다. 기존 `page_index`와 같은 0-based 규칙이다. 첫 페이지는 `[0, 1)`이다.
- Python에서는 `range`로 받는다: `pages=range(0, 1)`. `None`이면 전체 문서다.
- 범위가 비었거나(`start >= end`), 뒤집혔거나, 폭이 `MAX_PAGE_SPAN`(4096)을 넘으면 오류다. 범위 안의 모든 페이지에 결과 항목을 만들기 때문에 `0..u32::MAX` 같은 요청이 수십억 개의 항목을 할당하지 못하게 폭을 제한한다. 전체 문서는 범위 없이 요청한다.
- 문서 끝을 넘은 페이지는 오류가 아니라 `out_of_range` 항목으로 결과에 남는다. 일부만 유효한 요청을 표현할 수 있고, 오타가 조용한 빈 성공으로 보이는 일을 막는다.

### Result model

문서를 열 수 없는 경우는 예외가 아니라 결과의 `document_status`로 돌려준다. 호출 측은 예외 처리 없이 상태를 보고 다음 단계를 정할 수 있다. 예외는 호출 자체가 해석 불가능하거나(잘못된 범위, 잘못된 인자) 문서를 보기 전에 실패하는 경우(파일 I/O)에만 쓴다.

불변식:

- `document_status`가 `READY`가 아니면 `pages`는 비어 있고 `page_count`는 `None`이며 어떤 텍스트도 반환하지 않는다. 문서 앞부분 발췌를 페이지로 대신 내는 일은 구조적으로 불가능하다.
- 페이지의 `text`는 `status`가 `text`일 때만 비어 있지 않다.
- 요청 범위의 모든 페이지는 순서대로 결과에 남는다.
- 페이지 경계는 항상 조판으로 계산한 값이며 결과의 `pagination`이 `computed`로 표시한다. 한컴 오피스가 보여 주는 페이지와 다를 수 있는 추정값이다.

문서 상태 코드(`document_status`):

| Code | 의미 |
| --- | --- |
| `READY` | 문서를 열었고 페이지를 계산했다. `pages`를 쓸 수 있다. |
| `UNSUPPORTED_FORMAT` | 알 수 없는 형식(임의 바이트, PDF, HWPX가 아닌 ZIP 등). |
| `DRM_PROTECTED` | DRM/보안 컨테이너로 보호된 문서(복호화는 범위 밖). |
| `PASSWORD_REQUIRED` | 비밀번호가 필요한 문서. 비밀번호로 여는 경로는 이번 범위 밖이다. |
| `EMPTY_INPUT` | 0바이트 입력. |
| `UNPARSABLE` | 형식은 알지만 읽지 못했다. 사유는 `document_status_detail`. |
| `TOO_LARGE` | 입력이 크기 상한(256MB)을 넘었다. 파싱 전에 판정한다. |
| `PAGE_BOUNDARY_UNSUPPORTED` | 열 수는 있지만 페이지 경계를 검증하지 못한 형식(HML). 파싱 전에 거절하고 텍스트를 내지 않는다. |
| `NO_PAGES` | 열었지만 페이지가 0개다. |

페이지 상태 코드(`status`):

| Code | 의미 |
| --- | --- |
| `text` | 페이지에 텍스트가 있다. |
| `no_text` | 페이지를 읽었고 텍스트가 없다(이미지만 있는 페이지 등). 실패가 아니다. |
| `extraction_failed` | 이 페이지의 추출이 실패했다. 사유는 `status_detail`. 패닉도 이 페이지에서 격리한다. |
| `out_of_range` | 페이지가 문서 끝을 넘었다. |

요구사항 4상태와의 대응: 텍스트 없음은 `no_text`, 추출 실패는 `extraction_failed`(페이지) 또는 `UNPARSABLE`(문서), 페이지 경계 미지원은 `PAGE_BOUNDARY_UNSUPPORTED`, 잘못된 범위는 `InvalidPageRange` 오류(범위 자체가 잘못된 경우)와 `out_of_range`(문서 끝을 넘은 경우)다.

코드는 안정 토큰이다. 값 추가는 허용하되 기존 값의 이름 변경이나 삭제는 소비자를 깨뜨리므로 하지 않는다.

### Verified formats

페이지 텍스트를 내는 형식은 페이지 경계를 검증한 HWP 5.0, HWPX, HWP 3.0이다. HML은 샘플이 없어 검증하지 못했으므로 `PAGE_BOUNDARY_UNSUPPORTED`로 거절한다. HML 샘플로 측정한 뒤 허용할 수 있다.

형식은 확장자가 아니라 매직 바이트로 판별한다. `.hwpx` 확장자인데 실제로는 HWP 5.0 바이너리인 파일이 있다.

### Provenance

AGENTS.md의 시맨틱 객체 규칙(원본 식별, 페이지 인덱스, 바이트/해시, 아티팩트 키, 추출/변환 버전)에 대응한다.

- `source`: 입력 크기, 소문자 hex SHA-256, 감지한 형식(`hwp5`/`hwpx`/`hwp3`/`hml` 또는 `None`), 파일 API로 연 경우 파일 이름.
- 페이지 단위: `page_index`(0-based)와 `artifact_key = "{sha256}:p{page_index}:v{extraction_version}"`.
- 버전: `extraction_version`(정규화 규칙과 결과 의미가 바뀔 때 증가, 현재 1), `engine_version`(hwp-ingest crate 버전), `rhwp_version`(vendored rhwp 버전). `rhwp_version`은 `vendor/rhwp-subset/Cargo.toml`의 version과 같은지 검사하는 테스트가 drift를 막는다.

### Text normalization v1

최소한만 한다: `U+FFFC`(개체 자리표시) 제거, 앞뒤 공백 제거, 결과가 비면 `no_text`.

바꾸지 않는 것: 원문자(`①②`), 수식 스크립트 원문, 표 안 글자. 머리말·꼬리말은 `omit_header_footer=True`일 때만 제외한다. 정규화나 결과 의미를 바꾸면 `extraction_version`을 올린다.

## API

Rust engine:

```rust
pub struct TextOptions { pub pages: PageSelection, pub omit_header_footer: bool }
pub enum PageSelection { All, Range { start: u32, end: u32 } } // [start, end), 0-based

pub fn extract_text_bytes(data: &[u8], options: &TextOptions) -> Result<TextExtraction, HwpIngestError>;
pub fn extract_text_file(path: impl AsRef<Path>, options: &TextOptions) -> Result<TextExtraction, HwpIngestError>;
```

`HwpIngestError::InvalidPageRange { start, end }`는 잘못된 범위에, `HwpIngestError::Io`는 파일 읽기 실패에 쓴다. 문서 상태는 오류가 아니다.

Python facade:

```python
extract_text_bytes(data: bytes, *, pages: range | None = None, omit_header_footer: bool = False) -> TextExtraction
extract_text_file(input_path, *, pages: range | None = None, omit_header_footer: bool = False) -> TextExtraction
```

- 결과 `TextExtraction`과 `PageText`는 frozen이다. `pages`는 불변 튜플이다. 상태 값은 문자열 코드이고 스텁에 `Literal`로 선언한다.
- `pages`가 `range`가 아니면 `TypeError`, step이 1이 아니거나 음수이거나 32비트를 넘으면 `ValueError`다. 비었거나 뒤집혔거나 너무 넓은 범위는 `InvalidPageRangeError`다.
- `InvalidPageRangeError`는 `ValueError`의 하위 클래스다. 기존에 `ValueError`로 범위 오류를 처리하던 코드가 그대로 동작한다. `HwpIngestError`의 하위가 아니므로 범위 실수와 문서 실패가 구분된다.
- 파일이 없으면 `FileNotFoundError`다.

## Boundary rules

이 API는 장기 `rhwp` 내부 상태를 노출하지 않는다.

- `rhwp` 문서 AST, layout tree, 렌더 트리, 렌더러 소유 버퍼를 공개하지 않는다.
- adapter 소유 상태를 공개 Python 객체로 내놓지 않는다.
- 결과는 hwp-ingest가 소유한 compact 값(문자열, 숫자, 튜플)이다.
- 조판 정책, 상태 판정, 출처 정보는 Rust engine이 소유한다. Python은 변환 loop나 정책을 갖지 않는다.

## Known limits

- 페이지 경계는 추정값이다. 한컴 오피스와 한두 쪽 어긋날 수 있다(긴 문서의 뒤쪽에서 주로 발생). 첫 페이지 내용은 측정한 코퍼스에서 대부분 거의 같았지만 현장 문서에 대해서는 보증하지 않는다.
- 훼손된 파일이 오류 없이 열릴 수 있다. 소형 HWP는 파일 끝이 패딩이라 끝부분이 잘려도 전체 텍스트가 복구되고, 복구가 실패하는 지점 근처에서는 열기 성공과 실패가 뒤바뀐다. 호출 측이 훼손 여부를 알 방법은 없다.
- 수식은 수식 스크립트 원문으로, 원문자는 원문 그대로 나온다. 수식이나 양식이 많은 문서는 텍스트 표현이 달라질 수 있다. 머리말·꼬리말은 기본으로 포함되므로 반복 텍스트가 매 페이지 앞에 붙는다.
- 비밀번호로 열기, HML, 입력 크기 상한의 조정은 지원하지 않는다.
- GIL을 놓지 않는다. 기존 변환 함수와 같다.
- 배치 처리용 API는 이번 범위가 아니다. path-first 구조(`extract_text_file`)가 이후 batch 설계 자리를 남긴다.

## Non-goals

- OCR, 이미지 렌더링, 객체 저장소 다운로드, 분류 호출, 업무 DB 저장, 수집 정책.
- 문서 앞부분 발췌를 페이지 텍스트 대용으로 제공하는 것.
- 수식·표를 구조화하거나 Markdown으로 변환하는 것(후속 시맨틱 출력 설계 대상).
- 문서 전체를 한 번에 평탄화한 텍스트를 반환하는 것. 페이지를 식별할 수 없는 텍스트는 이 API에서 내지 않는다.

## Verification

- adapter 단위 테스트: 범위, 끝 초과, 잘못된 범위, HWP·HWPX 구조 일치, 텍스트 페이지 수와 SVG 페이지 수 일치, 머리말·꼬리말 제외, 읽을 수 없는 입력의 상태(빈 입력, 임의 바이트, PDF, 합성 DRM, 잘린 파일, HML), 오류 매핑, 정규화, `RHWP_VERSION` drift.
- engine 통합 테스트(`crates/hwp-ingest/tests/text.rs`): 같은 문서의 HWP·HWPX fixture 쌍으로 페이지별 텍스트 일치, 출처 해시와 `artifact_key`, 불변식, 상태 코드 안정성, 파일 API, 잘린 파일이 결과 계약을 깨지 않는지.
- Python 테스트(`tests/python/test_text.py`): 위 동작의 Python 표면, 불변성, PathLike, 스텁 커버리지(`__all__`, 패키지 스텁, native 모듈을 스텁과 비교).
- 코퍼스: upstream 샘플 842개로 열기 상태, 페이지 수, 빈 페이지 수를 측정하고 한컴 참조 PDF의 첫 페이지와 글자 다중집합 유사도로 대조했다.
- vendored 래퍼의 단위 테스트는 repo에 upstream 테스트 asset을 두지 않으므로 scratch 복사본에서 실행한다.

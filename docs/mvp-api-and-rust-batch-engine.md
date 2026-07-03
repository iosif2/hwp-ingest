# MVP API and Rust batch engine direction

<!-- Source: docs/rhwp-python-binding-plan.md lines 75-120. Original content below is copied verbatim. -->

## Current MVP API

현재 Python MVP API는 다음 signature를 유지한다.

```python
analyze_bytes(data: bytes) -> DocumentInfo
to_pdf_bytes(data: bytes, *, page_index: int | None = None) -> bytes
to_pdf_file(input_path, output_path=None, *, page_index=None, overwrite=False) -> Path
```

현재 Rust MVP shape는 다음과 같다.

```rust
analyze_hwp_bytes(data: &[u8])
hwp_to_pdf_bytes(data: &[u8], options: ConvertOptions)
hwp_file_to_pdf_file(input_path, output_path, options)
```

이 API들은 오늘 구현된 MVP surface다. `page_index`는 upstream native API와 같은 0-based page index다. `None`이면 전체 문서를 대상으로 한다. Python facade는 경로 정규화와 overwrite policy를 담당하고, Rust engine은 전달받은 data/path/options로 분석과 산출을 수행한다.

미래 batch API와 semantic API는 Rust engine crate에서 먼저 설계하고 구현한 뒤 PyO3 wrapper와 Python facade/stub으로 올라와야 한다. 현재 구현 위치는 `hwp-ingest`다. `rhwp` 고유 구조는 `hwp-ingest-rhwp-adapter`가 흡수한다. Python-only 확장으로 Rust engine이 생산하지 않는 semantic contract를 만들지 않는다.

## Rust library and batch engine direction

다른 Rust service는 Python 없이 `hwp-ingest` engine crate에 의존해 고성능 HWP/HWPX ingestion, rendered output generation, semantic artifact pipeline을 만들 수 있어야 한다. 현재 Rust MVP 사용 shape는 다음과 같다.

```rust
use hwp_ingest::{hwp_file_to_pdf_file, ConvertOptions};
```

이 import는 현재 MVP API 예시이며 final batch API가 아니다. `hwp-ingest-rhwp-adapter`는 engine 내부 backend adapter이므로 일반 Rust service는 직접 의존하지 않는다. 다음 symbols는 future API direction이다. 아직 존재하지 않으며, Python이 노출하기 전에 Rust engine crate에서 먼저 설계해야 한다.

```rust
pub struct Engine;
pub struct ConvertJob;
pub struct BatchOptions;
pub struct BatchResult;
```

Batch invariant는 `Python loop X, Rust loop O`다. Python `convert_many()`가 생기더라도 Python에서 파일별 loop를 돌며 FFI를 반복 호출하지 않는다. Python facade는 한 번 FFI boundary를 넘고, PyO3 layer는 `Python::allow_threads`로 GIL을 release하며, file-level concurrency와 scheduling은 Rust core/batch engine에서 수행한다.

Batch input/output은 path-first policy를 따른다. 대량 변환은 Python-owned `bytes`보다 Rust-owned path를 선호한다. 이렇게 해야 Python heap pressure를 줄이고, 이후 mmap, streaming write, path-output optimization을 적용할 수 있다. `to_pdf_bytes`는 small/interactive use를 위한 convenience API로 유지한다.

Conversion engine parallelism은 CPU-bound workload로 간주한다. 기본 선택은 sync Rust와 Rayon/threadpool이다. Tokio는 object storage, HTTP, queue consumer 같은 external async I/O 경계에서만 도입한다. CPU-bound conversion loop를 async task처럼 포장하지 않는다.

Memory lifetime policy는 compact result다. `BatchResult`는 document AST, layout tree, render buffer, borrowed upstream state를 반환하지 않는다. Batch 결과는 file/job-level report, artifact key/path, compact metadata, error만 담는다. Agent용 document/page/chunk object는 명시적인 semantic result object여야 하며 내부 `rhwp` state나 `hwp-ingest-rhwp-adapter` state를 borrow하지 않는다.

# hwp-ingest 아키텍처 방향

## 목적

`hwp-ingest`의 목표는 Python PDF binding 자체가 아니다. 최종 방향은 Rust-native HWP/HWPX ingestion and semantic conversion toolkit이다. 이 toolkit은 AI/agent pipeline이 원본 문서, 변환 산출물, 페이지/레이아웃 정보, semantic chunk provenance를 안정적으로 다루게 하고, Python은 그 위의 control-plane API를 제공한다.

PDF는 첫 번째 rendered output이자 현재 MVP의 PDF 변환 경로일 뿐 product boundary가 아니다. 기존 HWP/HWPX→PDF 중심 표현은 현재 동작을 설명할 때만 사용한다. 장기적으로 안전하게 노출할 수 있는 upstream `rhwp` output surface는 Rust core adapter를 통해 먼저 수용한다. 이미 확인된 방향은 PDF와 SVG/page rendering output이다. Text/Markdown-like extraction, layout/page metadata, table/shape regions, headings/sections 같은 surface는 유용한 대상이지만 unverified — confirm against rhwp public API before implementation. Agent-facing semantic objects는 upstream output과 hwp-ingest adapter가 생산할 수 있는 provenance만 담아야 한다.

## Architecture

```text
Python API / Engine facade = control plane
  ↓
PyO3 thin wrapper = type and exception conversion only
  ↓
hwp-ingest engine = product core, orchestration, batch policy
  ↓
hwp-ingest-rhwp-adapter = upstream rhwp shape → engine API conversion
  ↓
vendor/rhwp-subset = parser, layout, renderer, supported export surfaces
```

현재 repository 경계와 target crate naming은 분리해서 기록한다.

- `crates/hwp-ingest`: 제품의 진짜 Rust engine/core다. Python 없이도 쓰는 public Rust library boundary이며 batch orchestration, product API, semantic policy를 소유한다.
- `crates/hwp-ingest-rhwp-adapter`: vendored `rhwp` 구조를 hwp-ingest engine API로 변환하는 adapter다. upstream-derived code는 아니지만 upstream API 변화와 semantic/layout/render shape를 흡수한다.
- `crates/hwp-ingest-python`: PyO3 layer이며 target crate/package 이름도 `hwp-ingest-python`이다. Rust engine 타입과 오류를 Python 객체와 예외로 변환한다.
- `python/hwp_ingest`: Pythonic facade. `PathLike` 정규화, overwrite 정책, public re-export, type stub을 담당한다.

Rust engine은 PyO3-free로 유지한다. `hwp-ingest` Rust crate에는 `#[pyfunction]`, `#[pyclass]`, `pyo3` 타입, Python exception policy를 두지 않는다. Python API는 conversion loop, batch scheduling, document lifetime, semantic conversion policy를 소유하지 않는다. Python은 작업을 요청하고 결과를 받는 control plane이다.

Crate naming decision은 `hwp-ingest` + `hwp-ingest-rhwp-adapter` + `hwp-ingest-python`이다. `hwp-ingest`가 제품 core이고, `hwp-ingest-rhwp-adapter`는 현재 upstream backend인 `rhwp`를 engine boundary에 맞춘다. 이 분리를 유지해야 나중에 `rhwp` 말고 다른 parser/renderer backend를 붙일 수 있다.

선택된 upstream 연계 시작 아키텍처는 crates.io or git이 아니라 필요한 `rhwp` subset의 surgical vendoring이다.

Dependency policy는 다음 순서로 둔다.

1. 필요한 MIT-licensed `rhwp` subset을 `vendor/rhwp-subset`으로 가져온다.
2. hwp-ingest adapter/engine code는 upstream-derived code와 분리한다. `vendor/rhwp-subset`은 upstream 유래 코드, `hwp-ingest-rhwp-adapter`는 upstream 구조를 engine API로 바꾸는 코드, `hwp-ingest`는 제품 core다.
3. Upstream public API를 Python facade가 직접 보정하지 않는다. Parser, layout, renderer, export surface 변화는 `hwp-ingest-rhwp-adapter`에서 흡수하고, engine API는 제품 contract로 유지한다.

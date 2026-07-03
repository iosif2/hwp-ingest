# Typing and build workflow

<!-- Source: docs/rhwp-python-binding-plan.md lines 159-175. Original content below is copied verbatim. -->

## Typing policy

Python package는 checked-in `.pyi` files와 `py.typed`를 포함한다. Public Python API를 바꾸는 commit은 같은 commit에서 `python/hwp_ingest/__init__.pyi`, `python/hwp_ingest/_native.pyi`, 필요 시 facade implementation을 함께 갱신한다.

Batch/semantic API가 추가될 때 type stub은 Rust core가 실제로 생산하는 result object만 표현한다. Future-only symbols는 stub에 미리 추가하지 않는다.

## Build/test workflow

Local development 기본 명령은 다음과 같다.

```sh
uv run maturin develop
uv run pytest
cargo test --workspace
```

Wheel artifact는 GitHub Actions에서 빌드한다. CI는 Rust workspace test, maturin editable build, Python pytest를 먼저 실행한 뒤 Linux/macOS/Windows wheel을 생성한다.

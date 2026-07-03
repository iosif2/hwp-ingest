# hwp-ingest project instructions

This repository builds `hwp-ingest`: a Rust/Python HWP/HWPX ingestion toolkit. The first runnable product goal is HWP → PDF conversion, but the architecture must stay ready for rendered artifacts, text/layout extraction, and agent-facing semantic outputs.

## Read the project docs index first

Before changing architecture, dependency boundaries, conversion APIs, semantic outputs, or vendoring, consult `docs/rhwp-python-binding-plan.md` as the project docs index. Follow only the linked docs that are relevant to the change you are making.

Treat the docs reached through that index as the project-specific source of truth unless a newer user instruction explicitly overrides them.

## Architecture invariants

- `vendor/rhwp-subset` contains upstream-derived `rhwp` code only. Do not place hwp-ingest adapter, engine, Python, or product policy code in this tree.
- `crates/hwp-ingest-rhwp-adapter` is the only crate that translates vendored `rhwp` parser/layout/render structures into hwp-ingest-facing results. It may depend on `rhwp`; product-facing crates should not depend on `rhwp` directly.
- `crates/hwp-ingest` is the product engine/core. It owns public Rust API shape, conversion orchestration, file/byte behavior, future batch policy, semantic result policy, and engine-facing errors. It must stay PyO3-free.
- `crates/hwp-ingest-python` is the only Rust PyO3 layer. It owns native module registration and Rust-to-Python exception/type conversion only.
- `python/hwp_ingest` owns Pythonic naming, `PathLike` normalization, overwrite behavior, checked-in `.pyi` stubs, `py.typed`, and public re-exports.
- Python must not own conversion loops, batch scheduling, document lifetime, upstream API compensation, or semantic conversion policy.

## Vendor and upstream rules

- Keep `rhwp` license notices intact: MIT license copy, upstream third-party notices, root `NOTICE`, root `THIRD_PARTY_LICENSES.md`, and `licenses/rhwp-MIT.txt`.
- Keep vendor import and vendor modifications as separate commits when possible.
- Record origin URL, upstream license, copyright, imported tag/commit, imported date, imported scope, and local changes in `vendor/rhwp-subset/README.vendor.md`.
- For upstream sync, diff from the recorded vendor base, port parser/layout/render changes deliberately, rerun focused fixture regression checks, and update license metadata.
- If new upstream assets are needed for a feature, add only the required files and document why they belong in the vendor subset.

## API and semantic output rules

- Current MVP API is HWP → PDF through `analyze_bytes`, `to_pdf_bytes`, and `to_pdf_file` on Python, backed by `hwp-ingest` engine functions.
- New batch or semantic APIs must be designed in `crates/hwp-ingest` first, then exposed through PyO3 and Python stubs.
- Do not expose `rhwp` internal state, adapter-owned borrowed state, document ASTs, layout trees, or render buffers as long-lived public objects.
- Agent-facing semantic objects must carry provenance: source identity, page index when relevant, byte/hash identity when available, artifact path/key, extraction version, and conversion version.
- Prefer path-first batch APIs for large conversions. Keep bytes APIs as convenience APIs for small or interactive use.

## Typing and Python docstrings

- Every public Python symbol must have `.pyi` coverage.
- `python/hwp_ingest/py.typed` must remain packaged.
- Public Python functions and classes follow the global Korean Google-style docstring convention.

## Verification rule

Run focused local verification that matches the change before reporting completion. For the current HWP → PDF path, the lightweight checks are:

```sh
cargo test -p hwp-ingest full_conversion_returns_pdf_bytes
cargo check -p hwp-ingest-python
uv run maturin develop
uv run pytest tests/python/test_facade.py::test_to_pdf_bytes_returns_pdf_bytes
```

Use broader `cargo test --workspace` or full `uv run pytest` only when the change scope justifies it. Do not claim full integration coverage from a focused smoke check.

## Workflow and staging rules

- Do not stage `.github/` workflow files unless the user explicitly asks for workflow changes.
- Do not stage generated native extension artifacts such as `python/hwp_ingest/_native*.so`, `.pyd`, or `.dylib`.
- Stage changed files after implementation.
- Generate a commit message aligned with the currently staged contents and copy it to the tmux buffer when available.
- Do not create commits yourself.

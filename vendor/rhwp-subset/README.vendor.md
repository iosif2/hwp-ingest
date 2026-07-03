# rhwp vendored subset

## Origin

- Origin URL: https://github.com/edwardkim/rhwp
- Imported tag: v0.7.17
- Imported commit: 03351190ec35436e58cbfee0aa9278a8fdc04a59
- Imported date: 2026-07-03
- Upstream license: MIT
- Copyright: Copyright (c) 2025-2026 Edward Kim

## Imported scope

This vendor import contains the upstream Rust crate files required for hwp-ingest's current HWP to PDF path:

- `Cargo.toml`
- `src/`
- `saved/blank2010.hwp` required by an upstream `include_bytes!` path
- `examples/pr599_png_gateway.rs` required by an explicit upstream Cargo example target
- `LICENSE`
- `THIRD_PARTY_LICENSES.md`

Repository samples, web extension packages, studio apps, generated PDFs, and upstream CI/configuration files are intentionally not imported.

## Local changes

No upstream source files are modified in this import. hwp-ingest adapter, PyO3 binding, and Python facade code live outside `vendor/rhwp-subset`.

## Sync rule

The first vendor import preserves the imported upstream subset without source modification. Follow-up changes that prune or adapt upstream-derived code must be separate commits. Upstream sync starts by diffing from the recorded base commit, then ports parser/layout/render changes, reruns fixture regression checks, and updates this metadata plus license notices.

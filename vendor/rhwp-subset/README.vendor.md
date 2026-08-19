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
- `samples/hwpx/ref/ref_empty.hwpx` required by existing upstream serializer unit tests that are compiled during focused vendored-crate verification
- `LICENSE`
- `THIRD_PARTY_LICENSES.md`

Other repository samples, web extension packages, studio apps, generated PDFs, and upstream CI/configuration files are intentionally not imported.

## Local compliance additions

`NOTICE` is added by hwp-ingest. It repeats the Hancom HWP public-document notice and rhwp attribution inside the upstream-derived source subtree so source distributions retain the required notice near the vendored code.

The initial import preserved upstream source files without modification. Follow-up hwp-ingest-local source patches are tracked in `LOCAL_CHANGES.md`. hwp-ingest adapter, PyO3 binding, and Python facade code live outside `vendor/rhwp-subset`.

Current hwp-ingest-local parser/layout/render compatibility patches and conversion-time render option patches are recorded in LOCAL_CHANGES.md and must be reviewed during upstream sync.

## Selective upstream backports

Some `LOCAL_CHANGES.md` entries are not hwp-ingest-local inventions but selective backports of correctness fixes from later upstream `rhwp` commits, applied without a full resync. Each such entry names the upstream commit hash and issue/task number it ports. Because `vendor/rhwp-subset` does not carry upstream git history, these are adapted to this subset's baseline (`0335119`) rather than cherry-picked verbatim when the upstream code has diverged since.

Backported so far:

- `e893b65d9` (upstream Task #2220) — TAC host line `outer_margin` double-counting fix in `src/renderer/layout.rs`.

## Sync rule

The first vendor import preserves the imported upstream subset without source modification. Follow-up changes that prune or adapt upstream-derived code must be separate commits. Whenever parser/layout/render files are locally patched, update both this file and `LOCAL_CHANGES.md` in the same commit. Upstream sync starts by diffing from the recorded base commit, then ports parser/layout/render changes, reruns fixture regression checks, and updates this metadata plus license notices.

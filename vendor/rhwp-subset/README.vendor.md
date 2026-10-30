# rhwp vendored subset

## Origin

- Origin URL: https://github.com/edwardkim/rhwp
- Imported tag: v0.8.7
- Imported commit: 1a76570e833917d15817415a53c09ad61ab3203f
- Imported date: 2026-10-30
- Previous base: v0.7.17 (`03351190ec35436e58cbfee0aa9278a8fdc04a59`, imported 2026-07-03)
- Upstream license: MIT
- Copyright: Copyright (c) 2025-2026 Edward Kim

## Imported scope

This vendor import contains the upstream Rust crate files required for hwp-ingest's current HWP to PDF path:

- `Cargo.toml` (manifest adapted for a path dependency, see `LOCAL_CHANGES.md`)
- `build.rs`
- `src/`, excluding `src/bin/` (upstream CLI binaries that hwp-ingest does not use)
- `crates/rhwp-contracts`, `crates/rhwp-ooxml-chart`, `crates/rhwp-password-crypto` (path dependencies of the upstream crate)
- `saved/blank2010.hwp` required by an upstream `include_bytes!` path
- `samples/hwpx/ref/ref_empty.hwpx` required by existing upstream serializer unit tests
- `LICENSE`
- `THIRD_PARTY_LICENSES.md`

Upstream modules that hwp-ingest does not call (agent, crypto, rag, doclang, and similar) are kept unpruned so later syncs stay a straight replacement of `src/`.

Upstream unit tests under `src/` also `include_bytes!` other samples, fonts, and logos (about 16 MB, e.g. `samples/hwpx/aift.hwpx`, `ttfs/opensource/NotoSansKR-Regular.ttf`). They are intentionally not imported, so `cargo test -p rhwp --lib` does not compile in this tree. To run the vendored unit tests, copy the crate to a scratch directory, add an empty `[workspace]` table, and overlay the missing files from the upstream tag; the build errors name every missing path.

Other repository samples, web extension packages, studio apps, generated PDFs, upstream tests and examples, and upstream CI/configuration files are intentionally not imported.

## Local compliance additions

`NOTICE` is added by hwp-ingest. It repeats the Hancom HWP public-document notice and rhwp attribution inside the upstream-derived source subtree so source distributions retain the required notice near the vendored code.

Each import commit preserves upstream source files without modification. Follow-up hwp-ingest-local source patches are tracked in `LOCAL_CHANGES.md`. hwp-ingest adapter, PyO3 binding, and Python facade code live outside `vendor/rhwp-subset`.

Current hwp-ingest-local parser/layout/render compatibility patches (Hancom render compatibility shim, header/footer omission option, Paginator TAC remnant, page text extraction wrapper) and the manifest adaptation are recorded in LOCAL_CHANGES.md and must be reviewed during upstream sync.

## Selective upstream backports

None are currently carried. The earlier backports of upstream `e893b65d9` (Task #2220) and `c5f6f7278`/`a11997ea4` (Issue #1994) are contained in v0.8.7 and were removed during the v0.8.7 resync. If a future fix is backported without a full resync, record the upstream commit hash and issue/task number here and in `LOCAL_CHANGES.md`, adapted to this subset's baseline rather than cherry-picked verbatim when upstream has diverged.

## Sync rule

The first vendor import preserves the imported upstream subset without source modification. Follow-up changes that prune or adapt upstream-derived code must be separate commits. Whenever parser/layout/render files are locally patched, update both this file and `LOCAL_CHANGES.md` in the same commit. Upstream sync starts by diffing from the recorded base commit, then ports parser/layout/render changes, reruns fixture regression checks, and updates this metadata plus license notices.

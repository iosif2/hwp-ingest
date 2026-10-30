# hwp-ingest local changes for vendored rhwp subset

This file records hwp-ingest-local changes applied after the recorded upstream import because vendor/rhwp-subset is a surgical source subset and does not carry upstream git history. Keep one reverse-chronological entry per hwp-ingest commit that changes upstream-derived source files.

## 2026-10-30 — v0.8.7 resync: local patch inventory

Status: current hwp-ingest-local delta against unmodified upstream v0.8.7 (`1a76570e833917d15817415a53c09ad61ab3203f`). Production changes are about 90 lines plus `src/renderer/compat.rs`; everything else in this tree is upstream.

Retained (re-applied by hand onto the v0.8.7 layout, which was restructured since v0.7.17):

1. Hancom render compatibility shim — `src/renderer/compat.rs` (new), `LayoutEngine::{set_,}render_compatibility_options` and body-table clamp in `src/renderer/layout.rs`, `compute_table_y_position` clamp and `body_flow_context` plumbing in `src/renderer/layout/table_layout.rs`, `render_page_svg_native_with_compat` in `src/document_core/queries/rendering.rs` and `src/wasm_api.rs`, `pub mod compat` in `src/renderer/mod.rs`, and the CommonObjAttr flags write in `src/document_core/commands/table_ops.rs`. Also keeps the early `current_paper_width`/`current_body_area` set-and-restore in `build_render_tree`, so `is_body_flow_col_area` never reads the previous page's body area while headers/footers are laid out.
2. `RenderCompatibilityOptions::omit_header_footer` — header/footer omission in `build_render_tree` (`src/renderer/layout.rs`).
3. Paginator TAC remnant — `pre_table_end_line` in `src/renderer/pagination/engine.rs` no longer forces host text before a treat-as-char table when `vertical_offset > 0`.
4. Local regression tests for the above in `src/renderer/layout/tests.rs`, `src/renderer/pagination/tests.rs`, and `src/wasm_api/tests.rs`, plus unit tests in `compat.rs`.

Dropped because upstream v0.8.7 already contains an equivalent:

- TAC BehindText flow reservation: Paginator gate (`engine.rs`, upstream #1995), TypesetEngine (`typeset/controls/table_entry.rs`, `!treat_as_char`), and table layout (`table_layout.rs`, `!table.common.treat_as_char`). The `pre_table_end_line` case above is the only part upstream does not cover.
- Backport `e893b65d9` (upstream Task #2220, TAC host line `outer_margin` double-counting) and backports `c5f6f7278`/`a11997ea4` (upstream Issue #1994, Paper-anchored BehindText/InFrontOfText absolute placement). Both are ancestors of v0.8.7.

Rendering delta to expect: fixture SVGs differ from the v0.7.17 baseline (non-TAC table in the overlap fixture moves from y=210.96 to y=211.53; the TAC BehindText fixture table moves +1.86px in x) with vertical flow unchanged. These come from upstream layout changes, not from the local patches.

Verification: vendored unit tests run against a scratch copy with upstream test assets overlaid (the repo does not carry them): `renderer::compat::`, `hancom_render_compat`, `tac_behindtext`, `text_after_tac_behindtext`, `rhwp_native_compat`, `render_page_svg_native_with_compat` — 12 passed.

Upstream sync: Drop items 1-2 only if upstream exposes an equivalent transient render option; drop item 3 if upstream stops forcing host text before TAC tables with a vertical offset.

## 2026-10-30 — v0.8.7 resync: vendored manifest adaptation

Status: hwp-ingest-local build adaptation of the unmodified v0.8.7 import; no parser/layout/render source is changed.

Scope: `vendor/rhwp-subset/Cargo.toml` and stale subset files only. Upstream `src/` and `crates/rhwp-*` are untouched.

Problem: The upstream v0.8.7 manifest declares its own `[workspace]` (with tools/bindings members), 48 `[[test]]` and 12 `[[example]]` targets pointing at files that are not part of this subset, two `[[bin]]` targets (`rhwp`, `font-metric-gen`) whose `src/main.rs` `include_str!`s upstream docs (`llms.txt`, `mydocs/manual/*`), a `[patch.crates-io] svg2pdf` pointing at an unimported `vendor/svg2pdf`, and `[profile.*]` tables. The subset is a path dependency of the hwp-ingest workspace, so none of these can load or take effect there.

Behavior: Removed the nested `[workspace]`, all `[[test]]`/`[[example]]` targets, the `[[bin]]` targets, `[patch.crates-io]`, and `[profile.*]`; added `autobins = false`. The library target, dependencies, features, and lints are unchanged. `src/main.rs`, `src/tools/`, and other upstream files stay on disk unmodified. Removed the stale `examples/pr599_png_gateway.rs`, whose explicit upstream target no longer exists.

Verification: `cargo check -p rhwp --lib` passes against the unmodified v0.8.7 library source.

Files: `Cargo.toml`, `LOCAL_CHANGES.md`, `examples/pr599_png_gateway.rs` (deleted).

Upstream sync: Re-apply the same removals on every resync; the svg2pdf patch is intentionally not carried because hwp-ingest renders PDFs through its own `pdf_backend`.

## 2026-07-09 — render option for header/footer omission

Status: hwp-ingest-local render option; removable if upstream exposes an equivalent transient render option.

Scope: vendor/rhwp-subset transient render compatibility options and SVG render tree/cache handling used by hwp-ingest conversion policy; product adapter/core/Python API policy remains outside the vendored tree.

Problem: hwp-ingest needs a render-only conversion option that omits HWP Header/Footer control content, including images inside those controls, without changing document semantics, section/page-hide state, page layout, page count, master pages, borders/fill, or standalone page numbers.

Behavior: `RenderCompatibilityOptions::omit_header_footer` defaults to `false` for both `RHWP_NATIVE` and `HANCOM_RENDER_COMPATIBILITY`. When enabled for a render call, header/footer render tree emission is skipped and option-scoped SVG rendering invalidates the page tree cache before and after temporary option changes.

Files: `LOCAL_CHANGES.md`, `README.vendor.md`, `src/renderer/compat.rs`, `src/renderer/layout.rs`, and `src/document_core/queries/rendering.rs`.

Upstream sync: Default rhwp/native behavior remains unchanged; remove this local option only when upstream provides an equivalent non-mutating, transient render option for Header/Footer omission.

## 2026-07-06 — vendor-local Hancom render compatibility shim

Status: hwp-ingest-local compatibility shim; upstream contribution candidate.

Scope: vendor/rhwp-subset renderer compatibility API, table property round-trip support needed by compatibility regressions, and local documentation. The hwp-ingest adapter may opt into this vendor-local mode from outside `vendor/rhwp-subset`; product adapter/core/Python policy remains outside the vendored tree.

Problem: Hancom Office keeps body-flow order when an empty anchor paragraph contains a treat-as-char (TAC) table followed by a non-TAC `TopAndBottom` table. Upstream rhwp recorded the paragraph start y before laying out the TAC table, advanced the current flow y after the TAC table, then positioned the following non-TAC paragraph-relative table from the stale paragraph y plus `vertical_offset`. In the TAC/non-TAC table overlap regression fixture, this made the non-TAC table jump back upward and overlap the earlier TAC table, while Hancom Office stacks the non-TAC table after the TAC table. A related fallback also allowed non-overlay Paper/Page-relative body tables to render above current flow content.

Root cause: empty-anchor non-TAC table lane reservation used `para_y_for_table + vertical_offset` as the raw top even when previous TAC content in the same paragraph had already advanced `y_offset`. Compatibility placement must treat `y_offset` as the lower bound for non-overlay body-flow tables; otherwise the visual table bbox can be above the flow position that subsequent layout already assumes.

Behavior: Opt-in Hancom render compatibility clamps non-overlay body-flow non-TAC tables to the current flow anchor. For empty-anchor paragraph-relative `TopAndBottom` tables after TAC content, the raw lane top is `max(para_y + vertical_offset, current_flow_y)`, preventing TAC/non-TAC overlap while keeping upstream `RHWP_NATIVE` behavior unchanged by default.

Files: `LOCAL_CHANGES.md`, `README.vendor.md`, `samples/hwpx/ref/ref_empty.hwpx`, `src/renderer/compat.rs`, `src/renderer/mod.rs`, `src/renderer/layout.rs`, `src/renderer/layout/table_layout.rs`, `src/document_core/commands/table_ops.rs`, `src/document_core/queries/rendering.rs`, `src/wasm_api.rs`, `src/renderer/layout/tests.rs`, and `src/wasm_api/tests.rs`.

Upstream sync: Default `rhwp` behavior remains `RenderCompatibilityOptions::RHWP_NATIVE`; the local patch is removable when upstream provides equivalent Hancom-compatible table-flow placement.

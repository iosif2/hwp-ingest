# hwp-ingest local changes for vendored rhwp subset

This file records hwp-ingest-local changes applied after the recorded upstream import because vendor/rhwp-subset is a surgical source subset and does not carry upstream git history. Keep one reverse-chronological entry per hwp-ingest commit that changes upstream-derived source files.

## 2026-07-09 — render option for header/footer omission

Status: hwp-ingest-local render option; removable if upstream exposes an equivalent transient render option.

Scope: vendor/rhwp-subset transient render compatibility options and SVG render tree/cache handling used by hwp-ingest conversion policy; product adapter/core/Python API policy remains outside the vendored tree.

Problem: hwp-ingest needs a render-only conversion option that omits HWP Header/Footer control content, including images inside those controls, without changing document semantics, section/page-hide state, page layout, page count, master pages, borders/fill, or standalone page numbers.

Behavior: `RenderCompatibilityOptions::omit_header_footer` defaults to `false` for both `RHWP_NATIVE` and `HANCOM_RENDER_COMPATIBILITY`. When enabled for a render call, header/footer render tree emission is skipped and option-scoped SVG rendering invalidates the page tree cache before and after temporary option changes.

Files: `LOCAL_CHANGES.md`, `README.vendor.md`, `src/renderer/compat.rs`, `src/renderer/layout.rs`, and `src/document_core/queries/rendering.rs`.

Upstream sync: Default rhwp/native behavior remains unchanged; remove this local option only when upstream provides an equivalent non-mutating, transient render option for Header/Footer omission.

## 2026-07-07 — TAC BehindText table flow reservation

Status: hwp-ingest-local renderer compatibility fix; upstream contribution candidate.

Scope: vendor/rhwp-subset pagination/typeset/layout table classification and regression coverage, plus adapter fixture coverage for the public SVG/PDF path; product adapter/core/Python policy remains outside the vendored tree.

Problem: Treat-as-char tables saved with BehindText wrapping were classified as non-flowing Shape items before TAC handling in both pagination and typeset paths, and table layout also treated behind/front wrapping as no-advance regardless of TAC, so following body text rendered over the table.

Root cause: The BehindText/InFrontOfText table branches in Paginator::process_controls and TypesetEngine table placement ran before checking table.common.treat_as_char, and LayoutEngine::layout_table returned y_start for behind/front tables without excluding TAC tables.

Behavior: Non-TAC behind/front tables still render as overlay shapes with no flow advance, while TAC behind/front tables continue through table pagination/typesetting/layout and reserve flow space when non-inline.

Files: `LOCAL_CHANGES.md`, `README.vendor.md`, `src/renderer/pagination/engine.rs`, `src/renderer/typeset.rs`, `src/renderer/layout/table_layout.rs`, `src/renderer/pagination/tests.rs`, `src/renderer/layout/tests.rs`, `crates/hwp-ingest-rhwp-adapter/src/render.rs`, `tests/fixtures/tac-behindtext-table-flow.hwp`, and `tests/fixtures/README.md`.

Upstream sync: Remove this local patch only when upstream rhwp treats TAC tables as flow-reserving before applying behind/front overlay classification and layout no-advance rules.

## 2026-07-06 — vendor-local Hancom render compatibility shim

Status: hwp-ingest-local compatibility shim; upstream contribution candidate.

Scope: vendor/rhwp-subset renderer compatibility API, table property round-trip support needed by compatibility regressions, and local documentation. The hwp-ingest adapter may opt into this vendor-local mode from outside `vendor/rhwp-subset`; product adapter/core/Python policy remains outside the vendored tree.

Problem: Hancom Office keeps body-flow order when an empty anchor paragraph contains a treat-as-char (TAC) table followed by a non-TAC `TopAndBottom` table. Upstream rhwp recorded the paragraph start y before laying out the TAC table, advanced the current flow y after the TAC table, then positioned the following non-TAC paragraph-relative table from the stale paragraph y plus `vertical_offset`. In the TAC/non-TAC table overlap regression fixture, this made the non-TAC table jump back upward and overlap the earlier TAC table, while Hancom Office stacks the non-TAC table after the TAC table. A related fallback also allowed non-overlay Paper/Page-relative body tables to render above current flow content.

Root cause: empty-anchor non-TAC table lane reservation used `para_y_for_table + vertical_offset` as the raw top even when previous TAC content in the same paragraph had already advanced `y_offset`. Compatibility placement must treat `y_offset` as the lower bound for non-overlay body-flow tables; otherwise the visual table bbox can be above the flow position that subsequent layout already assumes.

Behavior: Opt-in Hancom render compatibility clamps non-overlay body-flow non-TAC tables to the current flow anchor. For empty-anchor paragraph-relative `TopAndBottom` tables after TAC content, the raw lane top is `max(para_y + vertical_offset, current_flow_y)`, preventing TAC/non-TAC overlap while keeping upstream `RHWP_NATIVE` behavior unchanged by default.

Files: `LOCAL_CHANGES.md`, `README.vendor.md`, `samples/hwpx/ref/ref_empty.hwpx`, `src/renderer/compat.rs`, `src/renderer/mod.rs`, `src/renderer/layout.rs`, `src/renderer/layout/table_layout.rs`, `src/document_core/commands/table_ops.rs`, `src/document_core/queries/rendering.rs`, `src/wasm_api.rs`, `src/renderer/layout/tests.rs`, and `src/wasm_api/tests.rs`.

Upstream sync: Default `rhwp` behavior remains `RenderCompatibilityOptions::RHWP_NATIVE`; the local patch is removable when upstream provides equivalent Hancom-compatible table-flow placement.

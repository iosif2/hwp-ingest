# Test fixtures

`atop-equation-01.hwp` is copied from `edwardkim/rhwp` tag `v0.7.17`, path `samples/atop-equation-01.hwp`, under the MIT license.

`tac-non-tac-table-overlap.hwp` is a maintainer-provided regression fixture added on 2026-07-06. It preserves a Hancom-compatible layout case where an empty anchor paragraph contains a treat-as-char table followed by a non-treat-as-char `TopAndBottom` table; rhwp-native placement overlapped the two tables, while Hancom Office stacks the non-TAC table after the TAC table.

`tac-behindtext-table-flow.hwp` is a maintainer-provided regression fixture added on 2026-07-07. It preserves a Hancom-compatible layout case where an empty anchor paragraph contains a wide treat-as-char table saved with `BehindText` wrapping; following body paragraphs must render below the table, not over it.

`footnote-01.hwp` and `footnote-01.hwpx` are the HWP 5.0 and HWPX versions of the same six-page document, copied unchanged from `edwardkim/rhwp` tag `v0.8.7`, paths `samples/footnote-01.hwp` and `samples/hwpx/footnote-01.hwpx`, under that repository's MIT license. They were added on 2026-10-30 as the multi-page fixtures for page-range text extraction: every page has text, both formats paginate to six pages, and extraction gives the same 2,035 characters from each. The repository licence covers the files as distributed upstream; the provenance of the article text inside the document is not documented upstream.

`hwp3-sample19-hwp5.hwp` and `hwp3-sample19-hwpx.hwpx` are HWP 5.0 and HWPX conversions of the same two-page document (upstream derived them from an HWP 3.0 sample), copied unchanged from `edwardkim/rhwp` tag `v0.8.7`, paths `samples/hwp3-sample19-hwp5.hwp` and `samples/hwp3-sample19-hwpx.hwpx`, under that repository's MIT license. They were added on 2026-10-30 because both carry header and footer text: extraction gives 1,012 characters per file, 888 when header and footer text is omitted. The content provenance of the document is not documented upstream.

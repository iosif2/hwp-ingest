# Test fixtures

`atop-equation-01.hwp` is copied from `edwardkim/rhwp` tag `v0.7.17`, path `samples/atop-equation-01.hwp`, under the MIT license.

`tac-non-tac-table-overlap.hwp` is a maintainer-provided regression fixture added on 2026-07-06. It preserves a Hancom-compatible layout case where an empty anchor paragraph contains a treat-as-char table followed by a non-treat-as-char `TopAndBottom` table; rhwp-native placement overlapped the two tables, while Hancom Office stacks the non-TAC table after the TAC table.

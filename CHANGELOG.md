# Changelog

## Unreleased

## 0.2.0

### Added

- Added page-range text extraction for HWP and HWPX: `extract_text_bytes` and `extract_text_file` return a `TextExtraction` with a document status, a status for every page, and provenance (source SHA-256, artifact keys, extraction/engine/rhwp versions). Unreadable documents are reported as a status instead of raising, and an invalid page range raises `InvalidPageRangeError`.
- Added a bundled Noto Sans KR (SIL OFL 1.1) fallback font for PDF output, so Hangul renders instead of missing-glyph boxes on systems with no Korean font. System fonts and `font_dirs` take precedence. Hanja, circled digits, fullwidth symbols, and Hancom private-use symbols are not covered.
- Added a `font_dirs` option to `to_pdf_bytes` and `to_pdf_file` (and the matching Rust `FontOptions` functions) to search extra font directories.

### Changed

- Resynced the vendored `rhwp` subset to v0.8.7. SVG/PDF output can differ slightly from 0.1.2 in layout details.
- The source distribution and wheels now include the bundled font and its license text, which adds about 2.5 MB.

## 0.1.2

### Added

- Added an `omit_header_footer` conversion option for SVG/PDF rendering so callers can omit HWP header/footer content without changing body layout or page selection.

### Fixed

- Fixed Hancom-compatible flow reservation for wide treat-as-char tables saved with BehindText wrapping, preventing following body text from rendering over the table in SVG/PDF output.
- Fixed body text after a treat-as-char table being pushed down and clipped when the table's outer margin was already accounted for in the saved layout, preventing double-counted spacing in SVG/PDF output.
- Fixed full-page Paper-anchored tables with BehindText/InFrontOfText wrapping being fragmented across multiple pages and overlapping earlier content, ensuring they render as a single absolutely-positioned table in SVG/PDF output.

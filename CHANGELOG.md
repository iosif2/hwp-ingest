# Changelog

## Unreleased

### Added

- Added an `omit_header_footer` conversion option for SVG/PDF rendering so callers can omit HWP header/footer content without changing body layout or page selection.

### Fixed

- Fixed Hancom-compatible flow reservation for wide treat-as-char tables saved with BehindText wrapping, preventing following body text from rendering over the table in SVG/PDF output.

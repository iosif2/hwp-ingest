# Changelog

## Unreleased

### Added

- Added an `omit_header_footer` conversion option for SVG/PDF rendering so callers can omit HWP header/footer content without changing body layout or page selection.

### Fixed

- Fixed Hancom-compatible flow reservation for wide treat-as-char tables saved with BehindText wrapping, preventing following body text from rendering over the table in SVG/PDF output.
- Fixed body text after a treat-as-char table being pushed down and clipped when the table's outer margin was already accounted for in the saved layout, preventing double-counted spacing in SVG/PDF output.

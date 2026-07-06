"""Pythonic facade for the native hwp-ingest extension."""

from os import PathLike
from pathlib import Path

from . import _native

DocumentInfo = _native.DocumentInfo
HwpIngestError = _native.HwpIngestError


def _validate_page_index(page_index: int | None) -> int | None:
    if page_index is not None and page_index < 0:
        raise ValueError("page_index must be zero-based and non-negative")

    return page_index


def analyze_bytes(data: bytes) -> DocumentInfo:
    """Analyze basic metadata from HWP document bytes.

    Args:
        data: Complete HWP document bytes.

    Returns:
        A `DocumentInfo` object containing page-count metadata.

    Raises:
        HwpIngestError: Raised when the HWP parser fails.
    """
    return _native.analyze_hwp_bytes(data)


def to_pdf_bytes(data: bytes, *, page_index: int | None = None) -> bytes:
    """Convert HWP document bytes into PDF bytes.

    `page_index` is zero-based. Omit it to convert the full document into one
    PDF. PDF conversion requires the `rsvg-convert` executable at runtime.

    Args:
        data: Complete HWP document bytes.
        page_index: Zero-based page number to convert. `None` converts the full document.

    Returns:
        PDF document bytes starting with `%PDF-`.

    Raises:
        ValueError: Raised when `page_index` is negative or outside the document range.
        HwpIngestError: Raised when parsing, rendering, or PDF backend execution fails.
    """
    return _native.hwp_to_pdf_bytes(data, _validate_page_index(page_index))


def to_pdf_file(
    input_path: str | PathLike[str],
    output_path: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
) -> Path:
    """Convert an HWP file into a PDF file.

    When `output_path` is omitted, the output path is the input path with a
    `.pdf` suffix. Existing files are protected by default; pass
    `overwrite=True` to replace them. PDF conversion requires the
    `rsvg-convert` executable at runtime.

    Args:
        input_path: Input HWP file path.
        output_path: Output PDF file path. `None` replaces the input suffix with `.pdf`.
        page_index: Zero-based page number to convert. `None` converts the full document.
        overwrite: Whether to replace an existing output file.

    Returns:
        Path to the created PDF file.

    Raises:
        FileExistsError: Raised when the output file exists and `overwrite` is `False`.
        FileNotFoundError: Raised when the input file does not exist.
        ValueError: Raised when `page_index` is negative or outside the document range.
        HwpIngestError: Raised when file I/O, parsing, rendering, PDF backend execution, or output writing fails.
    """
    input_file = Path(input_path)
    output_file = Path(output_path) if output_path is not None else input_file.with_suffix(".pdf")

    if output_file.exists() and not overwrite:
        raise FileExistsError(output_file)

    report = _native.hwp_to_pdf_file(
        str(input_file),
        str(output_file),
        _validate_page_index(page_index),
    )
    return Path(report.output_path)

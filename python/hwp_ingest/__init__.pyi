from os import PathLike
from pathlib import Path

from ._native import (
    DocumentInfo,
    HwpIngestError,
    InvalidPageRangeError,
    PageText,
    TextExtraction,
)

__all__ = [
    "DocumentInfo",
    "HwpIngestError",
    "InvalidPageRangeError",
    "PageText",
    "TextExtraction",
    "analyze_bytes",
    "extract_text_bytes",
    "extract_text_file",
    "to_pdf_bytes",
    "to_pdf_file",
    "to_svg_pages",
    "to_svg_files",
]


def analyze_bytes(data: bytes) -> DocumentInfo:
    """Analyze page-count metadata from HWP document bytes."""
    ...


def to_pdf_bytes(
    data: bytes,
    *,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> bytes:
    """Convert HWP document bytes into PDF bytes."""
    ...


def to_svg_pages(
    data: bytes,
    *,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> list[bytes]:
    """Render HWP document pages into SVG bytes."""
    ...


def to_pdf_file(
    input_path: str | PathLike[str],
    output_path: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
    omit_header_footer: bool = False,
) -> Path:
    """Convert an HWP file into a PDF file and return the created path."""
    ...


def to_svg_files(
    input_path: str | PathLike[str],
    output_dir: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
    omit_header_footer: bool = False,
) -> list[Path]:
    """Render HWP document pages into SVG files and return the created paths."""
    ...


def extract_text_bytes(
    data: bytes,
    *,
    pages: range | None = None,
    omit_header_footer: bool = False,
) -> TextExtraction:
    """Extract the text of a zero-based, half-open page range from HWP or HWPX bytes."""
    ...


def extract_text_file(
    input_path: str | PathLike[str],
    *,
    pages: range | None = None,
    omit_header_footer: bool = False,
) -> TextExtraction:
    """Extract the text of a zero-based, half-open page range from an HWP or HWPX file."""
    ...

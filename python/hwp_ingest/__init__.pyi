from os import PathLike
from pathlib import Path

from ._native import DocumentInfo, HwpIngestError

__all__ = ["DocumentInfo", "HwpIngestError", "analyze_bytes", "to_pdf_bytes", "to_pdf_file"]


def analyze_bytes(data: bytes) -> DocumentInfo:
    """Analyze page-count metadata from HWP document bytes."""
    ...


def to_pdf_bytes(data: bytes, *, page_index: int | None = None) -> bytes:
    """Convert HWP document bytes into PDF bytes."""
    ...


def to_pdf_file(
    input_path: str | PathLike[str],
    output_path: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
) -> Path:
    """Convert an HWP file into a PDF file and return the created path."""
    ...

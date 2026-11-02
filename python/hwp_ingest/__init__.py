"""Typed Python facade for HWP document analysis, PDF conversion, and page text extraction."""

from ._facade import (
    DocumentInfo,
    HwpIngestError,
    InvalidPageRangeError,
    PageText,
    TextExtraction,
    analyze_bytes,
    extract_text_bytes,
    extract_text_file,
    to_pdf_bytes,
    to_pdf_file,
    to_svg_files,
    to_svg_pages,
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

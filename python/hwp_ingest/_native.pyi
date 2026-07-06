class HwpIngestError(Exception):
    """Exception for HWP parsing, rendering, and PDF backend failures."""


class DocumentInfo:
    """Basic metadata for an analyzed HWP document."""

    @property
    def page_count(self) -> int:
        """Number of renderable pages in the document."""
        ...


class ConvertReport:
    """Result returned by file-based PDF conversion."""

    @property
    def output_path(self) -> str:
        """Path to the created PDF file."""
        ...

    @property
    def page_count(self) -> int:
        """Total page count of the input document."""
        ...

    @property
    def pages_converted(self) -> int:
        """Number of pages converted into the PDF."""
        ...

    @property
    def output_bytes(self) -> int:
        """Size of the created PDF file in bytes."""
        ...


def analyze_hwp_bytes(data: bytes) -> DocumentInfo:
    """Analyze basic metadata from HWP document bytes."""
    ...


def hwp_to_pdf_bytes(data: bytes, page_index: int | None = None) -> bytes:
    """Convert HWP document bytes into PDF bytes."""
    ...


def hwp_to_pdf_file(
    input_path: str,
    output_path: str,
    page_index: int | None = None,
) -> ConvertReport:
    """Convert an HWP file into a PDF file."""
    ...

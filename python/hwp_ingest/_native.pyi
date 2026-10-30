from typing import Literal

DocumentStatusCode = Literal[
    "READY",
    "UNSUPPORTED_FORMAT",
    "DRM_PROTECTED",
    "PASSWORD_REQUIRED",
    "EMPTY_INPUT",
    "UNPARSABLE",
    "TOO_LARGE",
    "PAGE_BOUNDARY_UNSUPPORTED",
    "NO_PAGES",
]
PageStatusCode = Literal["text", "no_text", "extraction_failed", "out_of_range"]
SourceFormatCode = Literal["hwp5", "hwpx", "hwp3", "hml"]


class HwpIngestError(Exception):
    """Exception for HWP parsing, rendering, and PDF backend failures."""


class InvalidPageRangeError(ValueError):
    """Raised when a text extraction page range is empty, inverted, or too wide."""


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


class PageText:
    """Text and status of one page in a text extraction result."""

    @property
    def page_index(self) -> int:
        """Zero-based page index in the document."""
        ...

    @property
    def status(self) -> PageStatusCode:
        """Page status: `text`, `no_text`, `extraction_failed`, or `out_of_range`."""
        ...

    @property
    def status_detail(self) -> str | None:
        """Failure reason when the status is `extraction_failed`, otherwise `None`."""
        ...

    @property
    def text(self) -> str:
        """Page text. Empty unless the status is `text`."""
        ...

    @property
    def artifact_key(self) -> str:
        """Stable key `{sha256}:p{page_index}:v{extraction_version}`."""
        ...


class TextExtraction:
    """Result of a page-range text extraction."""

    @property
    def is_ready(self) -> bool:
        """Whether the document was readable, so `pages` can be used."""
        ...

    @property
    def document_status(self) -> DocumentStatusCode:
        """Stable status code of the document."""
        ...

    @property
    def document_status_detail(self) -> str | None:
        """Extra detail for `UNPARSABLE` and `TOO_LARGE`, otherwise `None`."""
        ...

    @property
    def source_size_bytes(self) -> int:
        """Size of the input in bytes."""
        ...

    @property
    def source_sha256(self) -> str:
        """Lowercase hex SHA-256 of the input bytes."""
        ...

    @property
    def source_format(self) -> SourceFormatCode | None:
        """Detected format, or `None` when the input is not a recognized document."""
        ...

    @property
    def source_file_name(self) -> str | None:
        """File name when the input came from a file path, otherwise `None`."""
        ...

    @property
    def page_count(self) -> int | None:
        """Computed page count, or `None` when the document was not readable."""
        ...

    @property
    def pagination(self) -> Literal["computed"]:
        """How page boundaries were determined. HWP and HWPX store none, so they are computed."""
        ...

    @property
    def requested_pages(self) -> tuple[int, int] | None:
        """Requested `(start, end)` range, or `None` for the whole document."""
        ...

    @property
    def pages(self) -> tuple[PageText, ...]:
        """Per-page results in request order. Empty unless `is_ready`."""
        ...

    @property
    def extraction_version(self) -> int:
        """Version of the text normalization and result semantics."""
        ...

    @property
    def engine_version(self) -> str:
        """hwp-ingest engine version."""
        ...

    @property
    def rhwp_version(self) -> str:
        """Vendored rhwp version."""
        ...


def analyze_hwp_bytes(data: bytes) -> DocumentInfo:
    """Analyze basic metadata from HWP document bytes."""
    ...


def hwp_to_pdf_bytes(
    data: bytes,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> bytes:
    """Convert HWP document bytes into PDF bytes."""
    ...


def hwp_to_svg_pages(
    data: bytes,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> list[bytes]:
    """Render HWP document pages into SVG bytes."""
    ...


def hwp_to_pdf_file(
    input_path: str,
    output_path: str,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> ConvertReport:
    """Convert an HWP file into a PDF file."""
    ...


def hwp_to_svg_files(
    input_path: str,
    output_dir: str | None = None,
    page_index: int | None = None,
    overwrite: bool = False,
    omit_header_footer: bool = False,
) -> list[str]:
    """Render HWP document pages into SVG files."""
    ...


def extract_text_bytes(
    data: bytes,
    pages: tuple[int, int] | None = None,
    omit_header_footer: bool = False,
) -> TextExtraction:
    """Extract page-range text from HWP or HWPX bytes."""
    ...


def extract_text_file(
    input_path: str,
    pages: tuple[int, int] | None = None,
    omit_header_footer: bool = False,
) -> TextExtraction:
    """Extract page-range text from an HWP or HWPX file."""
    ...

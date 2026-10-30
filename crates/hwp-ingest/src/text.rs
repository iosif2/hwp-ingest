//! Page-range text extraction.
//!
//! The engine turns the adapter's page text into a result a caller can classify without parsing
//! messages: every document-level outcome is a [`DocumentStatus`] and every page-level outcome is a
//! [`PageStatus`], each with a stable code. Errors are reserved for requests that cannot be
//! interpreted (an invalid page range) or that fail before the document is seen (file I/O).
//!
//! Invariants of [`TextExtraction`]:
//! - If `document_status` is not [`DocumentStatus::Ready`], `pages` is empty and no text of any
//!   kind is returned, so text without a trustworthy page identity cannot be mistaken for a page.
//! - [`PageText::text`] is non-empty only for [`PageStatus::Text`].
//! - Pages past the end of the document stay in `pages` as [`PageStatus::OutOfRange`].
//! - Page boundaries are computed by layout, never stored in the file; see [`PaginationBasis`].

use std::path::Path;

use hwp_ingest_rhwp_adapter::text as adapter;
use sha2::{Digest, Sha256};

use crate::HwpIngestError;

/// Version of the text normalization and result semantics. It is part of every `artifact_key`, so
/// bump it whenever the text a caller would receive for the same bytes can change.
pub const EXTRACTION_VERSION: u32 = 1;

/// Widest page range one request may cover.
pub const MAX_PAGE_SPAN: u32 = adapter::MAX_PAGE_SPAN;

/// Which pages to extract. Pages are zero-based, matching `ConvertOptions::page_index`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PageSelection {
    /// Every page of the document.
    #[default]
    All,
    /// The half-open range `[start, end)`.
    Range { start: u32, end: u32 },
}

impl PageSelection {
    /// The first page, `[0, 1)`.
    pub const fn first_page() -> Self {
        Self::Range { start: 0, end: 1 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextOptions {
    pub pages: PageSelection,
    /// Leave header and footer text out of every page, like `ConvertOptions::omit_header_footer`.
    pub omit_header_footer: bool,
}

/// Formats whose pages the engine can identify, plus HML, which it recognizes but refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Hwp5,
    Hwpx,
    Hwp3,
    Hml,
}

impl SourceFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hwp5 => "hwp5",
            Self::Hwpx => "hwpx",
            Self::Hwp3 => "hwp3",
            Self::Hml => "hml",
        }
    }
}

/// How the pages of a result were determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationBasis {
    /// Computed by the layout engine. HWP and HWPX store no page boundaries, so a page can differ
    /// from the page Hancom Office shows; this is an estimate, not a guarantee.
    Computed,
}

impl PaginationBasis {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Computed => "computed",
        }
    }
}

/// Identity of the input bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdentity {
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 of the input bytes.
    pub sha256: String,
    /// `None` when the bytes are not a recognized document format.
    pub format: Option<SourceFormat>,
    /// Only set by [`extract_text_file`].
    pub file_name: Option<String>,
}

/// Versions needed to reproduce or invalidate a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance {
    pub extraction_version: u32,
    pub engine_version: &'static str,
    pub rhwp_version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentStatus {
    Ready,
    UnsupportedFormat,
    DrmProtected,
    PasswordRequired,
    EmptyInput,
    Unparsable {
        detail: String,
    },
    TooLarge {
        size_bytes: u64,
        limit_bytes: u64,
    },
    /// A format that opens but whose page boundaries are unverified. No text is returned.
    PageBoundaryUnsupported {
        format: SourceFormat,
    },
    NoPages,
}

impl DocumentStatus {
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Stable token for callers. Tokens may be added but never changed or removed.
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::DrmProtected => "DRM_PROTECTED",
            Self::PasswordRequired => "PASSWORD_REQUIRED",
            Self::EmptyInput => "EMPTY_INPUT",
            Self::Unparsable { .. } => "UNPARSABLE",
            Self::TooLarge { .. } => "TOO_LARGE",
            Self::PageBoundaryUnsupported { .. } => "PAGE_BOUNDARY_UNSUPPORTED",
            Self::NoPages => "NO_PAGES",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageStatus {
    /// The page has text.
    Text,
    /// The page was read and has no text, for example an image-only page. Not a failure.
    NoText,
    ExtractionFailed {
        detail: String,
    },
    /// The page lies past the end of the document.
    OutOfRange,
}

impl PageStatus {
    /// Stable token for callers. Tokens may be added but never changed or removed.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::NoText => "no_text",
            Self::ExtractionFailed { .. } => "extraction_failed",
            Self::OutOfRange => "out_of_range",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    /// Zero-based page index in the document.
    pub page_index: u32,
    pub status: PageStatus,
    /// Non-empty only when `status` is [`PageStatus::Text`].
    pub text: String,
    /// `"{sha256}:p{page_index}:v{extraction_version}"`.
    pub artifact_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextExtraction {
    pub document_status: DocumentStatus,
    pub source: SourceIdentity,
    /// Set only when the document opened and its pagination was computed.
    pub page_count: Option<u32>,
    pub pagination: PaginationBasis,
    pub requested: PageSelection,
    /// Empty unless `document_status` is [`DocumentStatus::Ready`].
    pub pages: Vec<PageText>,
    pub provenance: Provenance,
}

/// Extracts page text from in-memory document bytes.
///
/// # Errors
///
/// Returns [`HwpIngestError::InvalidPageRange`] when the requested range is empty, inverted, or
/// wider than [`MAX_PAGE_SPAN`]. Problems with the document itself are never errors; they are
/// reported through [`TextExtraction::document_status`].
pub fn extract_text_bytes(
    data: &[u8],
    options: &TextOptions,
) -> Result<TextExtraction, HwpIngestError> {
    extract(data, options, None)
}

/// Extracts page text from a file. Prefer this over reading the bytes yourself for large inputs.
///
/// # Errors
///
/// Returns [`HwpIngestError::Io`] when the file cannot be read, and the errors of
/// [`extract_text_bytes`].
pub fn extract_text_file(
    path: impl AsRef<Path>,
    options: &TextOptions,
) -> Result<TextExtraction, HwpIngestError> {
    let path = path.as_ref();
    let data = std::fs::read(path).map_err(|source| HwpIngestError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    extract(&data, options, file_name)
}

fn extract(
    data: &[u8],
    options: &TextOptions,
    file_name: Option<String>,
) -> Result<TextExtraction, HwpIngestError> {
    let request = adapter::TextRequest {
        range: match options.pages {
            PageSelection::All => None,
            PageSelection::Range { start, end } => Some(adapter::PageRange { start, end }),
        },
        omit_header_footer: options.omit_header_footer,
    };
    let outcome =
        adapter::extract_text(data, request).map_err(|error| HwpIngestError::InvalidPageRange {
            start: error.start,
            end: error.end,
        })?;

    let sha256 = sha256_hex(data);
    let format = outcome.format.map(source_format);
    let pages = outcome
        .pages
        .into_iter()
        .map(|page| PageText {
            artifact_key: artifact_key(&sha256, page.page_index),
            page_index: page.page_index,
            status: page_status(page.status),
            text: page.text,
        })
        .collect();

    Ok(TextExtraction {
        document_status: document_status(outcome.status, format),
        source: SourceIdentity {
            size_bytes: data.len() as u64,
            sha256,
            format,
            file_name,
        },
        page_count: outcome.page_count,
        pagination: PaginationBasis::Computed,
        requested: options.pages,
        pages,
        provenance: Provenance {
            extraction_version: EXTRACTION_VERSION,
            engine_version: env!("CARGO_PKG_VERSION"),
            rhwp_version: adapter::RHWP_VERSION,
        },
    })
}

fn artifact_key(sha256: &str, page_index: u32) -> String {
    format!("{sha256}:p{page_index}:v{EXTRACTION_VERSION}")
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn source_format(format: adapter::SourceFormat) -> SourceFormat {
    match format {
        adapter::SourceFormat::Hwp5 => SourceFormat::Hwp5,
        adapter::SourceFormat::Hwpx => SourceFormat::Hwpx,
        adapter::SourceFormat::Hwp3 => SourceFormat::Hwp3,
        adapter::SourceFormat::Hml => SourceFormat::Hml,
    }
}

fn document_status(
    status: adapter::TextDocumentStatus,
    format: Option<SourceFormat>,
) -> DocumentStatus {
    match status {
        adapter::TextDocumentStatus::Ready => DocumentStatus::Ready,
        adapter::TextDocumentStatus::UnsupportedFormat => DocumentStatus::UnsupportedFormat,
        adapter::TextDocumentStatus::DrmProtected => DocumentStatus::DrmProtected,
        adapter::TextDocumentStatus::PasswordRequired => DocumentStatus::PasswordRequired,
        adapter::TextDocumentStatus::EmptyInput => DocumentStatus::EmptyInput,
        adapter::TextDocumentStatus::Unparsable(detail) => DocumentStatus::Unparsable { detail },
        adapter::TextDocumentStatus::TooLarge {
            size_bytes,
            limit_bytes,
        } => DocumentStatus::TooLarge {
            size_bytes,
            limit_bytes,
        },
        // The adapter refuses only HML, and reports the detected format alongside the status.
        adapter::TextDocumentStatus::PageBoundaryUnsupported => {
            DocumentStatus::PageBoundaryUnsupported {
                format: format.unwrap_or(SourceFormat::Hml),
            }
        }
        adapter::TextDocumentStatus::NoPages => DocumentStatus::NoPages,
    }
}

fn page_status(status: adapter::TextPageStatus) -> PageStatus {
    match status {
        adapter::TextPageStatus::Text => PageStatus::Text,
        adapter::TextPageStatus::NoText => PageStatus::NoText,
        adapter::TextPageStatus::ExtractionFailed(detail) => {
            PageStatus::ExtractionFailed { detail }
        }
        adapter::TextPageStatus::OutOfRange => PageStatus::OutOfRange,
    }
}

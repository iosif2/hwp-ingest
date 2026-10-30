//! Page text extraction over the vendored `rhwp` crate.
//!
//! This module opens a document through `rhwp::service::DocumentService`, classifies why a document
//! cannot be read, and extracts the text of a page range. It deliberately does not share the open
//! path of the PDF/SVG conversion in [`crate::render`].
//!
//! Invariants the engine relies on:
//! - A document that is not [`TextDocumentStatus::Ready`] never yields any page, so a caller can
//!   never receive document text that is not tied to a page.
//! - Pages past the end of the document are reported as [`TextPageStatus::OutOfRange`] with empty
//!   text instead of failing the whole request.
//! - Page boundaries are computed by the layout engine, not stored in the file.

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use rhwp::parser::{FileFormat, detect_format};
use rhwp::renderer::compat::RenderCompatibilityOptions;
use rhwp::service::{DocumentService, OpenOptions, ServiceError};

/// Version of the vendored `rhwp` crate. Kept equal to `vendor/rhwp-subset/Cargo.toml` by a test.
pub const RHWP_VERSION: &str = "0.8.7";

/// Largest page range a single request may cover.
///
/// Every page of the range gets a result entry, including pages past the end of the document, so
/// the span is bounded to keep a request such as `0..u32::MAX` from allocating billions of entries.
pub const MAX_PAGE_SPAN: u32 = 4096;

const OBJECT_REPLACEMENT_CHAR: char = '\u{FFFC}';

/// Document formats whose page boundaries have been verified against reference output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Hwp5,
    Hwpx,
    Hwp3,
    /// Opens in `rhwp`, but its page boundaries have not been verified.
    Hml,
}

/// Zero-based, half-open page range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextRequest {
    /// `None` extracts every page of the document.
    pub range: Option<PageRange>,
    /// Leave header and footer text out of every page.
    pub omit_header_footer: bool,
}

/// A page range that is empty, inverted, or wider than [`MAX_PAGE_SPAN`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPageRange {
    pub start: u32,
    pub end: u32,
}

impl fmt::Display for InvalidPageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "page range [{}, {}) is empty, inverted, or wider than {MAX_PAGE_SPAN} pages",
            self.start, self.end
        )
    }
}

impl std::error::Error for InvalidPageRange {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextDocumentStatus {
    Ready,
    UnsupportedFormat,
    DrmProtected,
    PasswordRequired,
    EmptyInput,
    Unparsable(String),
    TooLarge { size_bytes: u64, limit_bytes: u64 },
    PageBoundaryUnsupported,
    NoPages,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextPageStatus {
    Text,
    NoText,
    ExtractionFailed(String),
    OutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPage {
    pub page_index: u32,
    pub status: TextPageStatus,
    /// Non-empty only when `status` is [`TextPageStatus::Text`].
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextOutcome {
    pub status: TextDocumentStatus,
    /// Detected format, when the bytes were recognized as one the document readers know.
    pub format: Option<SourceFormat>,
    /// Set only when the document opened and its pagination was computed.
    pub page_count: Option<u32>,
    /// Empty unless `status` is [`TextDocumentStatus::Ready`].
    pub pages: Vec<TextPage>,
}

impl TextOutcome {
    fn without_pages(status: TextDocumentStatus, format: Option<SourceFormat>) -> Self {
        Self {
            status,
            format,
            page_count: None,
            pages: Vec::new(),
        }
    }
}

/// Extracts the text of the requested pages.
///
/// Fails only when the requested range itself is invalid. Every problem with the document is
/// reported through [`TextOutcome::status`] instead.
pub fn extract_text(data: &[u8], request: TextRequest) -> Result<TextOutcome, InvalidPageRange> {
    if let Some(range) = request.range {
        validate_range(range)?;
    }

    let detected = detect_format(data);

    // HML opens in rhwp, but nothing verifies its pagination. Refuse before parsing so that no
    // text without a trustworthy page identity can leave this function.
    if detected == FileFormat::Hml {
        return Ok(TextOutcome::without_pages(
            TextDocumentStatus::PageBoundaryUnsupported,
            Some(SourceFormat::Hml),
        ));
    }

    let opened = match DocumentService::new().open_bytes(data, &OpenOptions::new()) {
        Ok(opened) => opened,
        Err(error) => {
            return Ok(TextOutcome::without_pages(
                classify_open_failure(&error),
                source_format(detected),
            ));
        }
    };

    let format = source_format(opened.format());
    let page_count = opened.page_count();
    if page_count == 0 {
        return Ok(TextOutcome::without_pages(
            TextDocumentStatus::NoPages,
            format,
        ));
    }

    let (start, end) = match request.range {
        Some(range) => (range.start, range.end),
        None => (0, page_count),
    };
    let options = RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY
        .with_omit_header_footer(request.omit_header_footer);

    let pages = (start..end)
        .map(|page_index| {
            if page_index >= page_count {
                return TextPage {
                    page_index,
                    status: TextPageStatus::OutOfRange,
                    text: String::new(),
                };
            }
            extract_page(page_index, || {
                opened
                    .core()
                    .extract_page_text_native_with_compat(page_index, options)
                    .map_err(|error| error.to_string())
            })
        })
        .collect();

    Ok(TextOutcome {
        status: TextDocumentStatus::Ready,
        format,
        page_count: Some(page_count),
        pages,
    })
}

fn validate_range(range: PageRange) -> Result<(), InvalidPageRange> {
    let invalid = InvalidPageRange {
        start: range.start,
        end: range.end,
    };
    match range.end.checked_sub(range.start) {
        Some(span) if span > 0 && span <= MAX_PAGE_SPAN => Ok(()),
        _ => Err(invalid),
    }
}

/// Runs one page extraction, keeping a failure or panic of that page from reaching the others.
fn extract_page(page_index: u32, extract: impl FnOnce() -> Result<String, String>) -> TextPage {
    let (status, text) = match catch_unwind(AssertUnwindSafe(extract)) {
        Ok(Ok(raw)) => {
            let text = normalize_page_text(&raw);
            if text.is_empty() {
                (TextPageStatus::NoText, text)
            } else {
                (TextPageStatus::Text, text)
            }
        }
        Ok(Err(detail)) => (TextPageStatus::ExtractionFailed(detail), String::new()),
        Err(_) => (
            TextPageStatus::ExtractionFailed("page text extraction panicked".to_string()),
            String::new(),
        ),
    };
    TextPage {
        page_index,
        status,
        text,
    }
}

/// Normalization v1: drop object replacement characters and surrounding whitespace.
///
/// Circled digits, equation scripts, and table text are intentionally left as extracted.
fn normalize_page_text(raw: &str) -> String {
    raw.chars()
        .filter(|ch| *ch != OBJECT_REPLACEMENT_CHAR)
        .collect::<String>()
        .trim()
        .to_string()
}

fn source_format(format: FileFormat) -> Option<SourceFormat> {
    match format {
        FileFormat::Hwp => Some(SourceFormat::Hwp5),
        FileFormat::Hwpx => Some(SourceFormat::Hwpx),
        FileFormat::Hwp3 => Some(SourceFormat::Hwp3),
        FileFormat::Hml => Some(SourceFormat::Hml),
        FileFormat::DrmProtected | FileFormat::Empty | FileFormat::Unknown => None,
    }
}

fn classify_open_failure(error: &ServiceError) -> TextDocumentStatus {
    match error {
        ServiceError::UnsupportedFormat { detected } => match detected {
            FileFormat::Empty => TextDocumentStatus::EmptyInput,
            FileFormat::DrmProtected => TextDocumentStatus::DrmProtected,
            _ => TextDocumentStatus::UnsupportedFormat,
        },
        ServiceError::PasswordRequired | ServiceError::PasswordMismatch => {
            TextDocumentStatus::PasswordRequired
        }
        ServiceError::TooLarge {
            size_bytes,
            limit_bytes,
        } => TextDocumentStatus::TooLarge {
            size_bytes: *size_bytes as u64,
            limit_bytes: *limit_bytes as u64,
        },
        // `open_bytes` never reads files, so `NotFound` and `Io` are not expected here; reporting
        // them as unparsable keeps the mapping total without inventing a new document state.
        ServiceError::Parse(_) | ServiceError::NotFound { .. } | ServiceError::Io { .. } => {
            TextDocumentStatus::Unparsable(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{RenderOptions, hwp_to_svg_pages};

    /// Builds a document whose body paragraphs sit on separate pages: `PAGE_n_TEXT` on page n.
    fn synthetic_hwp_bytes(pages: usize, header: Option<&str>, footer: Option<&str>) -> Vec<u8> {
        let mut doc = rhwp::wasm_api::HwpDocument::create_empty();
        doc.create_blank_document_native()
            .expect("blank template should load");
        for page in 0..pages {
            let label = format!("PAGE_{}_TEXT", page + 1);
            // The blank template's first paragraph starts at character offset 2.
            let offset = if page == 0 { 2 } else { 0 };
            doc.insert_text(0, page as u32, offset, &label)
                .expect("body text should insert");
            if page + 1 < pages {
                let at = offset + label.chars().count() as u32;
                doc.insert_page_break(0, page as u32, at)
                    .expect("page break should insert");
            }
        }
        if let Some(text) = header {
            doc.create_header_footer(0, true, 0)
                .expect("header should be created");
            doc.insert_text_in_header_footer(0, true, 0, 0, 0, text)
                .expect("header text should insert");
        }
        if let Some(text) = footer {
            doc.create_header_footer(0, false, 0)
                .expect("footer should be created");
            doc.insert_text_in_header_footer(0, false, 0, 0, 0, text)
                .expect("footer text should insert");
        }
        doc.export_hwp_native()
            .expect("synthetic HWP should serialize")
    }

    fn synthetic_hwpx_bytes(pages: usize) -> Vec<u8> {
        let hwp = synthetic_hwp_bytes(pages, None, None);
        let doc = rhwp::wasm_api::HwpDocument::from_bytes(&hwp)
            .expect("synthetic HWP should load for HWPX export");
        doc.export_hwpx().expect("synthetic HWPX should serialize")
    }

    fn all_pages() -> TextRequest {
        TextRequest::default()
    }

    fn range(start: u32, end: u32) -> TextRequest {
        TextRequest {
            range: Some(PageRange { start, end }),
            omit_header_footer: false,
        }
    }

    fn ready(data: &[u8], request: TextRequest) -> TextOutcome {
        let outcome = extract_text(data, request).expect("range should be valid");
        assert_eq!(outcome.status, TextDocumentStatus::Ready, "{outcome:?}");
        outcome
    }

    #[test]
    fn all_pages_are_extracted_in_order() {
        let outcome = ready(&synthetic_hwp_bytes(3, None, None), all_pages());

        assert_eq!(outcome.page_count, Some(3));
        assert_eq!(outcome.format, Some(SourceFormat::Hwp5));
        let indexes: Vec<u32> = outcome.pages.iter().map(|page| page.page_index).collect();
        assert_eq!(indexes, vec![0, 1, 2]);
        for (index, page) in outcome.pages.iter().enumerate() {
            assert_eq!(page.status, TextPageStatus::Text);
            assert!(
                page.text.contains(&format!("PAGE_{}_TEXT", index + 1)),
                "page {index}: {:?}",
                page.text
            );
        }
    }

    #[test]
    fn first_page_range_returns_only_the_first_page() {
        let outcome = ready(&synthetic_hwp_bytes(3, None, None), range(0, 1));

        assert_eq!(outcome.page_count, Some(3));
        assert_eq!(outcome.pages.len(), 1);
        assert_eq!(outcome.pages[0].page_index, 0);
        assert!(outcome.pages[0].text.contains("PAGE_1_TEXT"));
        assert!(!outcome.pages[0].text.contains("PAGE_2_TEXT"));
    }

    #[test]
    fn inner_range_keeps_absolute_page_indexes() {
        let outcome = ready(&synthetic_hwp_bytes(3, None, None), range(1, 3));

        let indexes: Vec<u32> = outcome.pages.iter().map(|page| page.page_index).collect();
        assert_eq!(indexes, vec![1, 2]);
        assert!(outcome.pages[0].text.contains("PAGE_2_TEXT"));
        assert!(outcome.pages[1].text.contains("PAGE_3_TEXT"));
    }

    #[test]
    fn pages_past_the_end_are_out_of_range_not_errors() {
        let outcome = ready(&synthetic_hwp_bytes(3, None, None), range(2, 5));

        assert_eq!(outcome.page_count, Some(3));
        let statuses: Vec<(u32, TextPageStatus)> = outcome
            .pages
            .iter()
            .map(|page| (page.page_index, page.status.clone()))
            .collect();
        assert_eq!(
            statuses,
            vec![
                (2, TextPageStatus::Text),
                (3, TextPageStatus::OutOfRange),
                (4, TextPageStatus::OutOfRange),
            ]
        );
        assert!(outcome.pages[1].text.is_empty());
        assert!(outcome.pages[2].text.is_empty());
    }

    #[test]
    fn a_range_entirely_past_the_end_lists_every_page_as_out_of_range() {
        let outcome = ready(&synthetic_hwp_bytes(2, None, None), range(7, 9));

        assert_eq!(outcome.pages.len(), 2);
        assert!(
            outcome
                .pages
                .iter()
                .all(|page| page.status == TextPageStatus::OutOfRange && page.text.is_empty())
        );
    }

    #[test]
    fn invalid_ranges_are_rejected_before_the_document_is_read() {
        let data = synthetic_hwp_bytes(1, None, None);
        for (start, end) in [
            (0, 0),
            (3, 3),
            (3, 1),
            (0, MAX_PAGE_SPAN + 1),
            (0, u32::MAX),
        ] {
            assert_eq!(
                extract_text(&data, range(start, end)),
                Err(InvalidPageRange { start, end }),
                "[{start}, {end})"
            );
        }
        // An invalid range is reported even when the document itself is unusable.
        assert!(extract_text(b"", range(2, 1)).is_err());
        assert!(extract_text(&data, range(0, MAX_PAGE_SPAN)).is_ok());
    }

    #[test]
    fn hwpx_matches_hwp_page_structure() {
        let hwp = ready(&synthetic_hwp_bytes(3, None, None), all_pages());
        let hwpx = ready(&synthetic_hwpx_bytes(3), all_pages());

        assert_eq!(hwpx.format, Some(SourceFormat::Hwpx));
        assert_eq!(hwpx.page_count, hwp.page_count);
        for (hwp_page, hwpx_page) in hwp.pages.iter().zip(hwpx.pages.iter()) {
            assert_eq!(hwpx_page.page_index, hwp_page.page_index);
            assert_eq!(hwpx_page.status, TextPageStatus::Text);
            assert_eq!(hwpx_page.text, hwp_page.text);
        }
    }

    #[test]
    fn text_page_count_matches_svg_page_count() {
        for data in [synthetic_hwp_bytes(3, None, None), synthetic_hwpx_bytes(3)] {
            let outcome = ready(&data, all_pages());
            let svg_pages =
                hwp_to_svg_pages(&data, RenderOptions::default()).expect("SVG render should work");

            assert_eq!(outcome.page_count, Some(svg_pages.len() as u32));
            assert_eq!(outcome.pages.len(), svg_pages.len());
        }
    }

    #[test]
    fn header_and_footer_text_can_be_omitted() {
        let data = synthetic_hwp_bytes(1, Some("HEADER_SENTINEL"), Some("FOOTER_SENTINEL"));

        let with = ready(&data, all_pages());
        assert!(with.pages[0].text.contains("PAGE_1_TEXT"));
        assert!(with.pages[0].text.contains("HEADER_SENTINEL"));
        assert!(with.pages[0].text.contains("FOOTER_SENTINEL"));

        let without = ready(
            &data,
            TextRequest {
                range: None,
                omit_header_footer: true,
            },
        );
        assert!(without.pages[0].text.contains("PAGE_1_TEXT"));
        assert!(!without.pages[0].text.contains("HEADER_SENTINEL"));
        assert!(!without.pages[0].text.contains("FOOTER_SENTINEL"));
    }

    #[test]
    fn a_page_without_text_is_no_text_not_a_failure() {
        let outcome = ready(&synthetic_hwp_bytes(0, None, None), all_pages());

        assert_eq!(outcome.page_count, Some(1));
        assert_eq!(outcome.pages[0].status, TextPageStatus::NoText);
        assert!(outcome.pages[0].text.is_empty());
    }

    #[test]
    fn unusable_bytes_report_a_document_status_and_no_pages() {
        let hwp = synthetic_hwp_bytes(3, None, None);
        let mut drm = b"\x9b DRMONE".to_vec();
        drm.extend_from_slice(&[0; 64]);
        let mut softcamp = b"SCDSA002".to_vec();
        softcamp.extend_from_slice(&[0; 64]);

        let cases: Vec<(&str, Vec<u8>, TextDocumentStatus)> = vec![
            ("empty", Vec::new(), TextDocumentStatus::EmptyInput),
            (
                "garbage",
                b"this is not a document".to_vec(),
                TextDocumentStatus::UnsupportedFormat,
            ),
            (
                "pdf",
                b"%PDF-1.7\n%....".to_vec(),
                TextDocumentStatus::UnsupportedFormat,
            ),
            ("fasoo drm", drm, TextDocumentStatus::DrmProtected),
            ("softcamp drm", softcamp, TextDocumentStatus::DrmProtected),
        ];
        for (label, data, expected) in cases {
            let outcome = extract_text(&data, range(0, 1)).expect("range is valid");
            assert_eq!(outcome.status, expected, "{label}");
            assert!(outcome.pages.is_empty(), "{label}");
            assert_eq!(outcome.page_count, None, "{label}");
        }

        let truncated = extract_text(&hwp[..hwp.len() / 2], range(0, 1)).expect("range is valid");
        assert!(
            matches!(truncated.status, TextDocumentStatus::Unparsable(_)),
            "{:?}",
            truncated.status
        );
        assert!(truncated.pages.is_empty());
        assert_eq!(truncated.format, Some(SourceFormat::Hwp5));
    }

    #[test]
    fn hml_is_refused_with_no_text_even_though_it_opens() {
        let hml =
            br#"<?xml version="1.0" encoding="UTF-8"?><HWPML Version="2.8"><BODY></BODY></HWPML>"#;

        let outcome = extract_text(hml, all_pages()).expect("range is valid");

        assert_eq!(outcome.status, TextDocumentStatus::PageBoundaryUnsupported);
        assert_eq!(outcome.format, Some(SourceFormat::Hml));
        assert_eq!(outcome.page_count, None);
        assert!(outcome.pages.is_empty());
    }

    #[test]
    fn open_failures_map_to_document_statuses() {
        use std::path::PathBuf;

        let cases = [
            (
                ServiceError::UnsupportedFormat {
                    detected: FileFormat::Empty,
                },
                TextDocumentStatus::EmptyInput,
            ),
            (
                ServiceError::UnsupportedFormat {
                    detected: FileFormat::DrmProtected,
                },
                TextDocumentStatus::DrmProtected,
            ),
            (
                ServiceError::UnsupportedFormat {
                    detected: FileFormat::Unknown,
                },
                TextDocumentStatus::UnsupportedFormat,
            ),
            (
                ServiceError::PasswordRequired,
                TextDocumentStatus::PasswordRequired,
            ),
            (
                ServiceError::PasswordMismatch,
                TextDocumentStatus::PasswordRequired,
            ),
            (
                ServiceError::TooLarge {
                    size_bytes: 300,
                    limit_bytes: 256,
                },
                TextDocumentStatus::TooLarge {
                    size_bytes: 300,
                    limit_bytes: 256,
                },
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(classify_open_failure(&error), expected, "{error:?}");
        }

        for error in [
            ServiceError::Parse("broken".to_string()),
            ServiceError::NotFound {
                path: PathBuf::from("missing.hwp"),
            },
        ] {
            assert!(
                matches!(
                    classify_open_failure(&error),
                    TextDocumentStatus::Unparsable(_)
                ),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_failing_or_panicking_page_does_not_affect_the_result_shape() {
        let failed = extract_page(4, || Err("layout failed".to_string()));
        assert_eq!(failed.page_index, 4);
        assert_eq!(
            failed.status,
            TextPageStatus::ExtractionFailed("layout failed".to_string())
        );
        assert!(failed.text.is_empty());

        let panicked = extract_page(5, || panic!("boom"));
        assert_eq!(panicked.page_index, 5);
        assert!(matches!(
            panicked.status,
            TextPageStatus::ExtractionFailed(_)
        ));
        assert!(panicked.text.is_empty());
    }

    #[test]
    fn normalization_drops_object_placeholders_and_trims() {
        assert_eq!(
            normalize_page_text("\u{FFFC} 제목\n본문\u{FFFC} \n"),
            "제목\n본문"
        );
        assert_eq!(normalize_page_text("\u{FFFC}\u{FFFC}  \n"), "");
        // Circled digits and equation script text are not rewritten.
        assert_eq!(normalize_page_text("①② a over b"), "①② a over b");
    }

    #[test]
    fn rhwp_version_matches_the_vendored_crate() {
        let manifest = include_str!("../../../vendor/rhwp-subset/Cargo.toml");
        let version = manifest
            .lines()
            .find_map(|line| line.strip_prefix("version = \""))
            .and_then(|rest| rest.strip_suffix('"'))
            .expect("vendored Cargo.toml should declare a package version");

        assert_eq!(RHWP_VERSION, version);
    }
}

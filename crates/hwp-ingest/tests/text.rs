use std::path::PathBuf;

use hwp_ingest::{
    ConvertOptions, DocumentStatus, EXTRACTION_VERSION, HwpIngestError, MAX_PAGE_SPAN,
    PageSelection, PageStatus, PaginationBasis, SourceFormat, TextExtraction, TextOptions,
    extract_text_bytes, extract_text_file, hwp_to_svg_pages,
};

const FOOTNOTE_HWP_SHA256: &str =
    "5bbc8a8fd23415aad59dd91d1eb261050946d9c64635933fcd49609ec2cc94e5";
const FOOTNOTE_HWPX_SHA256: &str =
    "2b59b7248af275a5fa6e108f95997ceacbec9f3bb4fc9b1a30502373a8f672cb";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn options(pages: PageSelection) -> TextOptions {
    TextOptions {
        pages,
        omit_header_footer: false,
    }
}

fn range(start: u32, end: u32) -> PageSelection {
    PageSelection::Range { start, end }
}

fn ready(data: &[u8], options: &TextOptions) -> TextExtraction {
    let result = extract_text_bytes(data, options).expect("request should be valid");
    assert_eq!(result.document_status, DocumentStatus::Ready, "{result:?}");
    result
}

fn total_chars(result: &TextExtraction) -> usize {
    result
        .pages
        .iter()
        .map(|page| page.text.chars().count())
        .sum()
}

/// Checks the contract every result must keep, whatever the input.
fn assert_invariants(result: &TextExtraction) {
    if !result.document_status.is_ready() {
        assert!(result.pages.is_empty(), "not ready, yet pages: {result:?}");
        assert_eq!(result.page_count, None, "{result:?}");
    }
    for page in &result.pages {
        assert_eq!(
            !page.text.is_empty(),
            page.status == PageStatus::Text,
            "text is non-empty exactly for Text pages: {page:?}"
        );
        assert_eq!(
            page.artifact_key,
            format!(
                "{}:p{}:v{EXTRACTION_VERSION}",
                result.source.sha256, page.page_index
            )
        );
    }
    let indexes: Vec<u32> = result.pages.iter().map(|page| page.page_index).collect();
    assert!(
        indexes.windows(2).all(|pair| pair[1] == pair[0] + 1),
        "{indexes:?}"
    );
    assert_eq!(result.pagination, PaginationBasis::Computed);
    assert_eq!(result.provenance.extraction_version, EXTRACTION_VERSION);
    assert_eq!(result.provenance.engine_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(result.source.sha256.len(), 64);
}

#[test]
fn first_page_is_text_for_hwp_and_hwpx() {
    for (name, format) in [
        ("footnote-01.hwp", SourceFormat::Hwp5),
        ("footnote-01.hwpx", SourceFormat::Hwpx),
    ] {
        let result = ready(&fixture(name), &options(PageSelection::first_page()));

        assert_invariants(&result);
        assert_eq!(result.source.format, Some(format), "{name}");
        assert_eq!(result.page_count, Some(6), "{name}");
        assert_eq!(result.requested, range(0, 1));
        assert_eq!(result.pages.len(), 1, "{name}");
        assert_eq!(result.pages[0].page_index, 0);
        assert_eq!(result.pages[0].status, PageStatus::Text);
        assert!(!result.pages[0].text.is_empty());
    }
}

#[test]
fn all_pages_match_between_hwp_and_hwpx() {
    let hwp = ready(&fixture("footnote-01.hwp"), &options(PageSelection::All));
    let hwpx = ready(&fixture("footnote-01.hwpx"), &options(PageSelection::All));

    assert_invariants(&hwp);
    assert_invariants(&hwpx);
    assert_eq!(hwp.pages.len(), 6);
    assert_eq!(hwpx.pages.len(), 6);
    assert_eq!(total_chars(&hwp), 2035);
    assert_eq!(total_chars(&hwpx), 2035);
    for (left, right) in hwp.pages.iter().zip(hwpx.pages.iter()) {
        assert_eq!(left.page_index, right.page_index);
        assert_eq!(left.text, right.text, "page {}", left.page_index);
    }
}

#[test]
fn inner_range_keeps_absolute_page_indexes() {
    let result = ready(&fixture("footnote-01.hwpx"), &options(range(1, 3)));

    assert_invariants(&result);
    let indexes: Vec<u32> = result.pages.iter().map(|page| page.page_index).collect();
    assert_eq!(indexes, vec![1, 2]);
    assert!(
        result
            .pages
            .iter()
            .all(|page| page.status == PageStatus::Text)
    );
}

#[test]
fn pages_past_the_end_stay_in_the_result_as_out_of_range() {
    let result = ready(&fixture("footnote-01.hwp"), &options(range(4, 8)));

    assert_invariants(&result);
    let statuses: Vec<(u32, &str)> = result
        .pages
        .iter()
        .map(|page| (page.page_index, page.status.code()))
        .collect();
    assert_eq!(
        statuses,
        vec![
            (4, "text"),
            (5, "text"),
            (6, "out_of_range"),
            (7, "out_of_range")
        ]
    );
    assert!(result.pages[2].text.is_empty() && result.pages[3].text.is_empty());
}

#[test]
fn invalid_ranges_are_errors() {
    let data = fixture("footnote-01.hwp");
    for (start, end) in [
        (0, 0),
        (2, 2),
        (2, 1),
        (0, MAX_PAGE_SPAN + 1),
        (0, u32::MAX),
    ] {
        let error = extract_text_bytes(&data, &options(range(start, end)))
            .expect_err("range should be rejected");
        assert!(
            matches!(error, HwpIngestError::InvalidPageRange { start: s, end: e } if s == start && e == end),
            "[{start}, {end}): {error:?}"
        );
        assert!(error.to_string().contains(&format!("[{start}, {end})")));
    }
    // A range at the cap is accepted.
    assert!(extract_text_bytes(&data, &options(range(0, MAX_PAGE_SPAN))).is_ok());
}

#[test]
fn source_identity_is_the_hash_of_the_input_bytes() {
    let hwp = fixture("footnote-01.hwp");
    let hwpx = fixture("footnote-01.hwpx");

    let first = ready(&hwp, &options(PageSelection::first_page()));
    assert_eq!(first.source.sha256, FOOTNOTE_HWP_SHA256);
    assert_eq!(first.source.size_bytes, hwp.len() as u64);
    assert_eq!(first.source.file_name, None);
    assert_eq!(
        first.pages[0].artifact_key,
        format!("{FOOTNOTE_HWP_SHA256}:p0:v1")
    );

    let second = ready(&hwpx, &options(PageSelection::first_page()));
    assert_eq!(second.source.sha256, FOOTNOTE_HWPX_SHA256);

    // The same bytes and options always give the same result.
    assert_eq!(first, ready(&hwp, &options(PageSelection::first_page())));
}

#[test]
fn provenance_names_the_engine_and_the_vendored_rhwp() {
    let result = ready(
        &fixture("footnote-01.hwp"),
        &options(PageSelection::first_page()),
    );

    assert_eq!(result.provenance.extraction_version, 1);
    assert_eq!(result.provenance.engine_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(result.provenance.rhwp_version, "0.8.7");
}

#[test]
fn omit_header_footer_removes_header_and_footer_text() {
    for name in ["hwp3-sample19-hwp5.hwp", "hwp3-sample19-hwpx.hwpx"] {
        let data = fixture(name);

        let with = ready(&data, &options(PageSelection::All));
        let without = ready(
            &data,
            &TextOptions {
                pages: PageSelection::All,
                omit_header_footer: true,
            },
        );

        assert_invariants(&with);
        assert_invariants(&without);
        assert_eq!(with.page_count, Some(2), "{name}");
        assert_eq!(without.page_count, Some(2), "{name}");
        assert_eq!(total_chars(&with), 1012, "{name}");
        assert_eq!(total_chars(&without), 888, "{name}");
    }
}

#[test]
fn text_pages_follow_the_same_pagination_as_conversion() {
    for name in [
        "footnote-01.hwp",
        "footnote-01.hwpx",
        "hwp3-sample19-hwp5.hwp",
    ] {
        let data = fixture(name);

        let result = ready(&data, &options(PageSelection::All));
        let svg_pages = svg_page_count(&data);

        assert_eq!(result.page_count, Some(svg_pages as u32), "{name}");
        assert_eq!(result.pages.len(), svg_pages, "{name}");
    }
}

fn svg_page_count(data: &[u8]) -> usize {
    hwp_to_svg_pages(data, ConvertOptions::default())
        .expect("conversion should still work")
        .len()
}

#[test]
fn unusable_inputs_report_a_status_and_never_any_text() {
    let hwp = fixture("footnote-01.hwp");
    let mut fasoo = b"\x9b DRMONE".to_vec();
    fasoo.extend_from_slice(&[0; 64]);
    let hml: &[u8] =
        br#"<?xml version="1.0" encoding="UTF-8"?><HWPML Version="2.8"><BODY></BODY></HWPML>"#;

    let cases: Vec<(&str, Vec<u8>, &str, Option<SourceFormat>)> = vec![
        ("empty", Vec::new(), "EMPTY_INPUT", None),
        (
            "garbage",
            b"this is not a document".to_vec(),
            "UNSUPPORTED_FORMAT",
            None,
        ),
        ("drm", fasoo, "DRM_PROTECTED", None),
        (
            "truncated hwp (header lost)",
            hwp[..hwp.len() / 10].to_vec(),
            "UNPARSABLE",
            Some(SourceFormat::Hwp5),
        ),
        (
            "truncated hwp (quarter left)",
            hwp[..hwp.len() / 4].to_vec(),
            "UNPARSABLE",
            Some(SourceFormat::Hwp5),
        ),
        (
            "hml",
            hml.to_vec(),
            "PAGE_BOUNDARY_UNSUPPORTED",
            Some(SourceFormat::Hml),
        ),
    ];

    for (label, data, code, format) in cases {
        for pages in [PageSelection::All, PageSelection::first_page()] {
            let result = extract_text_bytes(&data, &options(pages)).expect("range is valid");

            assert_invariants(&result);
            assert_eq!(result.document_status.reason_code(), code, "{label}");
            assert_eq!(result.source.format, format, "{label}");
            assert_eq!(result.source.size_bytes, data.len() as u64, "{label}");
            assert_eq!(result.requested, pages, "{label}");
        }
    }

    let empty = extract_text_bytes(&[], &options(PageSelection::All)).expect("range is valid");
    assert_eq!(empty.source.sha256, EMPTY_SHA256);

    let hml_result = extract_text_bytes(hml, &options(PageSelection::All)).expect("range is valid");
    assert_eq!(
        hml_result.document_status,
        DocumentStatus::PageBoundaryUnsupported {
            format: SourceFormat::Hml
        }
    );
}

/// A truncated file may still open: the reader recovers what it can, and the tail of a small HWP is
/// often padding. Near the point where recovery stops working the outcome flips between `READY` and
/// `UNPARSABLE`, so this test pins only what must hold either way, not which side a given cut falls
/// on. Callers cannot detect that a document that opens was damaged.
#[test]
fn truncated_files_never_break_the_result_contract() {
    let hwp = fixture("footnote-01.hwp");

    for percent in [100, 90, 75, 60, 55, 50, 45, 25, 10] {
        let cut = &hwp[..hwp.len() * percent / 100];
        let result = extract_text_bytes(cut, &options(PageSelection::All)).expect("range is valid");

        assert_invariants(&result);
        assert!(
            matches!(
                result.document_status,
                DocumentStatus::Ready | DocumentStatus::Unparsable { .. }
            ),
            "{percent}%: {:?}",
            result.document_status
        );
        if result.document_status.is_ready() {
            assert_eq!(result.page_count, Some(6), "{percent}%");
        }
    }
}

#[test]
fn an_invalid_range_is_reported_even_for_an_unusable_document() {
    assert!(matches!(
        extract_text_bytes(&[], &options(range(2, 1))),
        Err(HwpIngestError::InvalidPageRange { start: 2, end: 1 })
    ));
}

#[test]
fn file_api_records_the_file_name_and_matches_the_bytes_api() {
    let path = fixture_path("footnote-01.hwpx");
    let data = fixture("footnote-01.hwpx");
    let request = options(range(0, 2));

    let from_file = extract_text_file(&path, &request).expect("file should extract");
    let from_bytes = extract_text_bytes(&data, &request).expect("bytes should extract");

    assert_eq!(
        from_file.source.file_name.as_deref(),
        Some("footnote-01.hwpx")
    );
    assert_eq!(from_bytes.source.file_name, None);
    assert_eq!(from_file.pages, from_bytes.pages);
    assert_eq!(from_file.source.sha256, from_bytes.source.sha256);
    assert_eq!(from_file.document_status, from_bytes.document_status);
}

#[test]
fn file_api_reports_a_missing_file_as_an_io_error() {
    let error = extract_text_file(fixture_path("does-not-exist.hwp"), &TextOptions::default())
        .expect_err("a missing file is an I/O error, not a document status");

    assert!(
        matches!(
            &error,
            HwpIngestError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound
        ),
        "{error:?}"
    );
}

#[test]
fn status_codes_are_stable() {
    let statuses = [
        (DocumentStatus::Ready, "READY"),
        (DocumentStatus::UnsupportedFormat, "UNSUPPORTED_FORMAT"),
        (DocumentStatus::DrmProtected, "DRM_PROTECTED"),
        (DocumentStatus::PasswordRequired, "PASSWORD_REQUIRED"),
        (DocumentStatus::EmptyInput, "EMPTY_INPUT"),
        (
            DocumentStatus::Unparsable {
                detail: String::new(),
            },
            "UNPARSABLE",
        ),
        (
            DocumentStatus::TooLarge {
                size_bytes: 2,
                limit_bytes: 1,
            },
            "TOO_LARGE",
        ),
        (
            DocumentStatus::PageBoundaryUnsupported {
                format: SourceFormat::Hml,
            },
            "PAGE_BOUNDARY_UNSUPPORTED",
        ),
        (DocumentStatus::NoPages, "NO_PAGES"),
    ];
    for (status, code) in statuses {
        assert_eq!(status.reason_code(), code);
        assert_eq!(status.is_ready(), code == "READY");
    }

    let pages = [
        (PageStatus::Text, "text"),
        (PageStatus::NoText, "no_text"),
        (
            PageStatus::ExtractionFailed {
                detail: String::new(),
            },
            "extraction_failed",
        ),
        (PageStatus::OutOfRange, "out_of_range"),
    ];
    for (status, code) in pages {
        assert_eq!(status.code(), code);
    }

    assert_eq!(SourceFormat::Hwp5.as_str(), "hwp5");
    assert_eq!(SourceFormat::Hwpx.as_str(), "hwpx");
    assert_eq!(SourceFormat::Hwp3.as_str(), "hwp3");
    assert_eq!(SourceFormat::Hml.as_str(), "hml");
    assert_eq!(PaginationBasis::Computed.as_str(), "computed");
}

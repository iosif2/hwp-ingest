use hwp_ingest::{ConvertOptions, HwpIngestError, analyze_hwp_bytes, hwp_to_pdf_bytes};

fn fixture_bytes() -> Vec<u8> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/atop-equation-01.hwp");

    std::fs::read(fixture).expect("fixture should be readable")
}

#[test]
fn analyze_reports_page_count() {
    let data = fixture_bytes();

    let info = analyze_hwp_bytes(&data).expect("fixture should parse");

    assert!(info.page_count > 0);
}

#[test]
fn full_conversion_returns_pdf_bytes() {
    let data = fixture_bytes();

    let pdf = hwp_to_pdf_bytes(&data, ConvertOptions::default()).expect("fixture should render");

    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 1000);
}

#[test]
fn single_page_conversion_returns_pdf_bytes() {
    let data = fixture_bytes();

    let pdf = hwp_to_pdf_bytes(
        &data,
        ConvertOptions {
            page_index: Some(0),
        },
    )
    .expect("fixture page should render");

    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn page_count_is_out_of_range_page_index() {
    let data = fixture_bytes();
    let info = analyze_hwp_bytes(&data).expect("fixture should parse");

    let error = hwp_to_pdf_bytes(
        &data,
        ConvertOptions {
            page_index: Some(info.page_count),
        },
    )
    .expect_err("page_count is one past the last zero-based page");

    assert!(matches!(
        error,
        HwpIngestError::PageOutOfRange {
            requested,
            page_count,
        } if requested == info.page_count && page_count == info.page_count
    ));
}

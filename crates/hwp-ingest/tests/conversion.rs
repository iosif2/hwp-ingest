use std::io::ErrorKind;

use hwp_ingest::{
    ConvertOptions, FontOptions, HwpIngestError, analyze_hwp_bytes,
    hwp_file_to_pdf_file_with_fonts, hwp_file_to_svg_files, hwp_to_pdf_bytes,
    hwp_to_pdf_bytes_with_fonts, hwp_to_svg_pages,
};

fn fixture_bytes() -> Vec<u8> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/atop-equation-01.hwp");

    std::fs::read(fixture).expect("fixture should be readable")
}

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/atop-equation-01.hwp")
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

fn temp_svg_dir(test_name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hwp-ingest-{test_name}-{}-{nanos}",
        std::process::id()
    ))
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
            ..ConvertOptions::default()
        },
    )
    .expect("fixture page should render");

    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn font_options_default_to_no_extra_directories() {
    assert!(FontOptions::default().font_dirs.is_empty());
}

#[test]
fn pdf_with_fonts_converts_with_and_without_extra_font_directories() {
    let data = fixture_bytes();
    let font_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../hwp-ingest-rhwp-adapter/assets/fonts");

    let plain =
        hwp_to_pdf_bytes_with_fonts(&data, ConvertOptions::default(), &FontOptions::default())
            .expect("default fonts should render");
    let with_dir = hwp_to_pdf_bytes_with_fonts(
        &data,
        ConvertOptions::default(),
        &FontOptions {
            font_dirs: vec![font_dir],
        },
    )
    .expect("an extra font directory should render");

    assert!(plain.starts_with(b"%PDF-"));
    assert!(with_dir.starts_with(b"%PDF-"));
    // The original API keeps working and goes through the same path as the default fonts.
    let original = hwp_to_pdf_bytes(&data, ConvertOptions::default()).expect("should render");
    assert!(original.starts_with(b"%PDF-"));
}

#[test]
fn pdf_file_with_fonts_writes_a_pdf_and_reports_pages() {
    let output_dir = temp_svg_dir("pdf-fonts");
    std::fs::create_dir_all(&output_dir).expect("output directory should be created");
    let output = output_dir.join("out.pdf");

    let report = hwp_file_to_pdf_file_with_fonts(
        fixture_path(),
        &output,
        ConvertOptions::default(),
        &FontOptions::default(),
    )
    .expect("PDF file should be written");

    assert_eq!(report.output_path, output);
    assert_eq!(report.page_count, 1);
    assert!(
        std::fs::read(&output)
            .expect("PDF should exist")
            .starts_with(b"%PDF-")
    );
    let _ = std::fs::remove_dir_all(output_dir);
}

#[test]
fn page_count_is_out_of_range_page_index() {
    let data = fixture_bytes();
    let info = analyze_hwp_bytes(&data).expect("fixture should parse");

    let error = hwp_to_pdf_bytes(
        &data,
        ConvertOptions {
            page_index: Some(info.page_count),
            ..ConvertOptions::default()
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

#[test]
fn svg_pages_return_svg_documents() {
    let data = fixture_bytes();
    let info = analyze_hwp_bytes(&data).expect("fixture should parse");

    let pages = hwp_to_svg_pages(&data, ConvertOptions::default()).expect("fixture should render");

    assert_eq!(pages.len(), info.page_count as usize);
    assert!(
        pages
            .iter()
            .all(|page| trim_ascii_start(&page.svg).starts_with(b"<svg"))
    );
    assert!(pages.iter().all(|page| {
        page.svg
            .windows(b"</svg>".len())
            .any(|window| window == b"</svg>")
    }));
    assert_eq!(pages[0].page_index, 0);
}

#[test]
fn single_page_svg_matches_first_page() {
    let data = fixture_bytes();

    let all_pages =
        hwp_to_svg_pages(&data, ConvertOptions::default()).expect("fixture should render");
    let single = hwp_to_svg_pages(
        &data,
        ConvertOptions {
            page_index: Some(0),
            ..ConvertOptions::default()
        },
    )
    .expect("fixture page should render");

    assert_eq!(single.len(), 1);
    assert_eq!(single[0], all_pages[0]);
}

#[test]
fn svg_conversion_reuses_page_out_of_range_error() {
    let data = fixture_bytes();
    let info = analyze_hwp_bytes(&data).expect("fixture should parse");

    let error = hwp_to_svg_pages(
        &data,
        ConvertOptions {
            page_index: Some(info.page_count),
            ..ConvertOptions::default()
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

#[test]
fn svg_files_write_stable_page_paths() {
    let output_dir = temp_svg_dir("svg-files-write");

    let paths = hwp_file_to_svg_files(
        fixture_path(),
        Some(&output_dir),
        ConvertOptions {
            page_index: Some(0),
            ..ConvertOptions::default()
        },
        false,
    )
    .expect("fixture page should write SVG");

    assert_eq!(
        paths,
        vec![output_dir.join("atop-equation-01.page-0001.svg")]
    );
    assert!(paths[0].exists());
    assert!(
        trim_ascii_start(&std::fs::read(&paths[0]).expect("SVG output should be readable"))
            .starts_with(b"<svg")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn svg_files_refuse_overwrite() {
    let output_dir = temp_svg_dir("svg-files-overwrite");
    std::fs::create_dir_all(&output_dir).expect("temp output dir should be creatable");
    let existing = output_dir.join("atop-equation-01.page-0001.svg");
    std::fs::write(&existing, b"existing").expect("existing output should be writable");

    let error = hwp_file_to_svg_files(
        fixture_path(),
        Some(&output_dir),
        ConvertOptions {
            page_index: Some(0),
            ..ConvertOptions::default()
        },
        false,
    )
    .expect_err("existing output should be protected");

    assert!(matches!(
        error,
        HwpIngestError::Io { path, source }
            if path == existing && source.kind() == ErrorKind::AlreadyExists
    ));

    let _ = std::fs::remove_dir_all(&output_dir);
}

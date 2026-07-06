use crate::{RhwpAdapterError, parse_hwp_bytes, pdf_backend};
use rhwp::renderer::compat::RenderCompatibilityOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgPage {
    pub page_index: u32,
    pub svg: String,
}

pub fn hwp_to_svg_pages(
    data: &[u8],
    page_index: Option<u32>,
) -> Result<Vec<SvgPage>, RhwpAdapterError> {
    let document = parse_hwp_bytes(data)?;
    let page_count = document.inner.page_count();

    if page_count == 0 {
        return Err(RhwpAdapterError::EmptyDocument);
    }

    if let Some(page) = page_index {
        if page >= page_count {
            return Err(RhwpAdapterError::PageOutOfRange {
                requested: page,
                page_count,
            });
        }
    }

    let pages: Vec<u32> = match page_index {
        Some(page) => vec![page],
        None => (0..page_count).collect(),
    };

    let mut svg_pages = Vec::with_capacity(pages.len());
    for page in pages {
        let svg = document
            .inner
            .render_page_svg_native_with_compat(
                page,
                RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            )
            .map_err(|error| RhwpAdapterError::Render(error.to_string()))?;
        svg_pages.push(SvgPage {
            page_index: page,
            svg,
        });
    }

    Ok(svg_pages)
}

pub fn hwp_to_pdf_bytes(data: &[u8], page_index: Option<u32>) -> Result<Vec<u8>, RhwpAdapterError> {
    let svg_pages = hwp_to_svg_pages(data, page_index)?;
    let svg_documents: Vec<String> = svg_pages.into_iter().map(|page| page.svg).collect();
    pdf_backend::svgs_to_pdf(&svg_documents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_usize(json: &str, key: &str) -> usize {
        let needle = format!("\"{key}\":");
        let start = json
            .find(&needle)
            .unwrap_or_else(|| panic!("{key} missing from JSON: {json}"))
            + needle.len();
        let digits: String = json[start..]
            .chars()
            .skip_while(|character| character.is_ascii_whitespace())
            .take_while(|character| character.is_ascii_digit())
            .collect();

        digits
            .parse()
            .unwrap_or_else(|_| panic!("{key} should be a usize in JSON: {json}"))
    }

    fn synthetic_non_overlay_paper_table_hwp_bytes() -> Vec<u8> {
        let mut doc = rhwp::wasm_api::HwpDocument::create_empty();
        doc.create_blank_document_native()
            .expect("synthetic blank template should load");
        let created = doc
            .create_table_ex(
                r#"{"sectionIdx":0,"paraIdx":0,"charOffset":2,"rowCount":1,"colCount":1,"treatAsChar":false,"colWidths":[6000],"rowHeights":[1200]}"#,
            )
            .expect("synthetic non-TAC table should be created");
        let para_idx = json_usize(&created, "paraIdx");
        let control_idx = json_usize(&created, "controlIdx");

        doc.set_table_properties(
            0,
            para_idx as u32,
            control_idx as u32,
            r#"{"treatAsChar":false,"textWrap":"TopAndBottom","vertRelTo":"Paper","vertAlign":"Top","vertOffset":0,"horzRelTo":"Paper","horzAlign":"Left","horzOffset":0,"restrictInPage":false,"allowOverlap":false}"#,
        )
        .expect("synthetic table positioning should be set");

        doc.export_hwp_native()
            .expect("synthetic HWP should serialize")
    }

    fn tac_non_tac_overlap_fixture_bytes() -> Vec<u8> {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("adapter crate should live under workspace crates directory")
            .join("tests/fixtures/tac-non-tac-table-overlap.hwp");
        std::fs::read(&fixture)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture.display()))
    }

    fn tac_behindtext_flow_fixture_bytes() -> Vec<u8> {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("adapter crate should live under workspace crates directory")
            .join("tests/fixtures/tac-behindtext-table-flow.hwp");
        std::fs::read(&fixture)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture.display()))
    }

    fn svg_attr_f64(element: &str, attr: &str) -> f64 {
        let needle = format!(r#"{attr}=""#);
        let start = element
            .find(&needle)
            .unwrap_or_else(|| panic!("{attr} missing from SVG element: {element}"))
            + needle.len();
        let end = element[start..]
            .find('"')
            .unwrap_or_else(|| panic!("{attr} value not closed in SVG element: {element}"))
            + start;
        element[start..end]
            .parse()
            .unwrap_or_else(|_| panic!("{attr} should be numeric in SVG element: {element}"))
    }

    fn first_svg_text_y(svg: &str, text: &str) -> f64 {
        let needle = format!(">{text}</text>");
        let text_end = svg
            .find(&needle)
            .unwrap_or_else(|| panic!("SVG text {text:?} missing"));
        let text_start = svg[..text_end]
            .rfind("<text")
            .unwrap_or_else(|| panic!("SVG text {text:?} has no opening text element"));
        svg_attr_f64(&svg[text_start..text_end], "y")
    }

    fn tac_behindtext_fixture_table_bottom_y(svg: &str) -> f64 {
        let table_left = r#"x1="75.58666666666667""#;
        let table_right = r#"x2="701.5066666666667""#;
        let bottom = svg
            .match_indices("<line")
            .filter_map(|(start, _)| {
                let end = svg[start..].find('>')? + start + 1;
                let line = &svg[start..end];
                (line.contains(table_left) && line.contains(table_right))
                    .then(|| svg_attr_f64(line, "y1"))
            })
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            bottom.is_finite(),
            "TAC BehindText table border line missing"
        );
        bottom
    }

    #[test]
    fn hwp_to_svg_pages_uses_hancom_render_compatibility() {
        let bytes = synthetic_non_overlay_paper_table_hwp_bytes();
        let adapter_pages =
            hwp_to_svg_pages(&bytes, Some(0)).expect("adapter SVG render should succeed");

        assert_eq!(adapter_pages.len(), 1);
        assert_eq!(adapter_pages[0].page_index, 0);

        let compat_svg = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
            .expect("synthetic HWP should parse for compat render")
            .render_page_svg_native_with_compat(
                0,
                RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            )
            .expect("direct compat render should succeed");

        let native_svg = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
            .expect("synthetic HWP should parse for native render")
            .render_page_svg_native(0)
            .expect("direct native render should succeed");

        assert_ne!(
            compat_svg, native_svg,
            "synthetic document must exercise the compatibility path"
        );
        assert_eq!(adapter_pages[0].svg, compat_svg);
    }

    #[test]
    fn hwp_to_svg_pages_resolves_tac_non_tac_overlap_fixture() {
        let bytes = tac_non_tac_overlap_fixture_bytes();
        let adapter_pages =
            hwp_to_svg_pages(&bytes, Some(0)).expect("adapter SVG render should succeed");
        assert_eq!(adapter_pages.len(), 1);
        assert_eq!(adapter_pages[0].page_index, 0);

        let native_svg = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
            .expect("overlap fixture should parse for native render")
            .render_page_svg_native(0)
            .expect("direct native render should succeed");

        assert_ne!(
            adapter_pages[0].svg, native_svg,
            "overlap fixture must exercise Hancom compatibility placement"
        );
        assert!(
            !adapter_pages[0].svg.contains(r#"y1="120.84""#),
            "adapter output must not keep the overlapping non-TAC table at the stale paragraph y"
        );
        assert!(
            adapter_pages[0].svg.contains(r#"y1="210.96""#),
            "adapter output should stack the non-TAC table after the preceding TAC table"
        );
    }

    #[test]
    fn hwp_to_svg_pages_reserves_flow_for_tac_behindtext_fixture() {
        let bytes = tac_behindtext_flow_fixture_bytes();
        let adapter_pages =
            hwp_to_svg_pages(&bytes, Some(0)).expect("adapter SVG render should succeed");
        assert_eq!(adapter_pages.len(), 1);
        assert_eq!(adapter_pages[0].page_index, 0);

        let svg = &adapter_pages[0].svg;
        let table_bottom_y = tac_behindtext_fixture_table_bottom_y(svg);
        let heading_y = first_svg_text_y(svg, "○");

        assert!(
            heading_y > table_bottom_y + 1.0,
            "following body heading should render below the TAC BehindText table; \
             heading_y={heading_y}, table_bottom_y={table_bottom_y}"
        );
        assert!(svg.contains(">현</text>"));
        assert!(svg.contains(">황</text>"));
    }

    #[test]
    #[ignore = "writes SVG artifacts for manual Hancom compatibility inspection"]
    fn write_hancom_compat_visual_svg_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = synthetic_non_overlay_paper_table_hwp_bytes();
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("adapter crate should live under workspace crates directory");
        let output_dir = workspace_root.join("target/hwp-ingest-visual");
        std::fs::create_dir_all(&output_dir)?;

        let native_path = output_dir.join("hancom-table-flow-rhwp-native.svg");
        let compat_path = output_dir.join("hancom-table-flow-hancom-compat.svg");
        let adapter_path = output_dir.join("hancom-table-flow-adapter.svg");
        let display_dir = std::path::Path::new("target/hwp-ingest-visual");
        let native_display = display_dir.join("hancom-table-flow-rhwp-native.svg");
        let compat_display = display_dir.join("hancom-table-flow-hancom-compat.svg");
        let adapter_display = display_dir.join("hancom-table-flow-adapter.svg");

        let native_svg = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
            .expect("synthetic HWP should parse for native render")
            .render_page_svg_native(0)
            .expect("direct native render should succeed");
        let compat_svg = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
            .expect("synthetic HWP should parse for compat render")
            .render_page_svg_native_with_compat(
                0,
                RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            )
            .expect("direct compat render should succeed");
        let mut adapter_pages = hwp_to_svg_pages(&bytes, Some(0))?;
        let adapter_svg = adapter_pages.remove(0).svg;

        assert_ne!(
            compat_svg, native_svg,
            "synthetic document must exercise the compatibility path"
        );
        assert_eq!(adapter_svg, compat_svg);

        std::fs::write(&native_path, native_svg)?;
        std::fs::write(&compat_path, compat_svg)?;
        std::fs::write(&adapter_path, adapter_svg)?;

        println!("{}", native_display.display());
        println!("{}", compat_display.display());
        println!("{}", adapter_display.display());

        Ok(())
    }
}

use crate::{RhwpAdapterError, parse_hwp_bytes, pdf_backend};

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
            .render_page_svg_native(page)
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

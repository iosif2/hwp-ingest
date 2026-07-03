use crate::{RhwpAdapterError, parse_hwp_bytes, pdf_backend};

pub fn hwp_to_pdf_bytes(data: &[u8], page_index: Option<u32>) -> Result<Vec<u8>, RhwpAdapterError> {
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
        svg_pages.push(
            document
                .inner
                .render_page_svg_native(page)
                .map_err(|error| RhwpAdapterError::Render(error.to_string()))?,
        );
    }

    pdf_backend::svgs_to_pdf(&svg_pages)
}

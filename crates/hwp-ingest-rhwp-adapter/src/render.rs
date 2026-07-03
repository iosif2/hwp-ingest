use crate::{RhwpAdapterError, parse_hwp_bytes};

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

        return document
            .inner
            .render_pages_pdf_native(&[page])
            .map_err(|error| RhwpAdapterError::Render(error.to_string()));
    }

    document
        .inner
        .render_document_pdf_native()
        .map_err(|error| RhwpAdapterError::Render(error.to_string()))
}

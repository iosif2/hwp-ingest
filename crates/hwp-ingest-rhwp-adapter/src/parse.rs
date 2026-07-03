use rhwp::wasm_api::HwpDocument;

use crate::{DocumentLayoutInfo, RhwpAdapterError};

pub struct ParsedHwpDocument {
    pub(crate) inner: HwpDocument,
}

impl ParsedHwpDocument {
    pub fn layout_info(&self) -> DocumentLayoutInfo {
        DocumentLayoutInfo {
            page_count: self.inner.page_count(),
        }
    }
}

pub fn parse_hwp_bytes(data: &[u8]) -> Result<ParsedHwpDocument, RhwpAdapterError> {
    HwpDocument::from_bytes(data)
        .map(|inner| ParsedHwpDocument { inner })
        .map_err(|error| RhwpAdapterError::Parse(error.to_string()))
}

pub fn analyze_hwp_bytes(data: &[u8]) -> Result<DocumentLayoutInfo, RhwpAdapterError> {
    parse_hwp_bytes(data).map(|document| document.layout_info())
}

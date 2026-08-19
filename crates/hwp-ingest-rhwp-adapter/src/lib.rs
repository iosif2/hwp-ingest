pub mod layout;
pub mod parse;
pub(crate) mod pdf_backend;
pub mod render;

pub use layout::DocumentLayoutInfo;
pub use parse::{analyze_hwp_bytes, parse_hwp_bytes};
pub use render::{RenderOptions, SvgPage, hwp_to_pdf_bytes, hwp_to_svg_pages};

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum RhwpAdapterError {
    Parse(String),
    Render(String),
    PdfBackendUnavailable(String),
    EmptyDocument,
    PageOutOfRange { requested: u32, page_count: u32 },
}

impl fmt::Display for RhwpAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "failed to parse HWP document: {message}"),
            Self::Render(message) => write!(f, "failed to render document: {message}"),
            Self::PdfBackendUnavailable(message) => write!(f, "{message}"),
            Self::EmptyDocument => write!(f, "HWP document contains no pages"),
            Self::PageOutOfRange {
                requested,
                page_count,
            } => write!(
                f,
                "page index {requested} is out of range for document with {page_count} pages"
            ),
        }
    }
}

impl Error for RhwpAdapterError {}

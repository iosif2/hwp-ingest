use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use hwp_ingest_rhwp_adapter::RhwpAdapterError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConvertOptions {
    pub page_index: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentInfo {
    pub page_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertReport {
    pub output_path: PathBuf,
    pub page_count: u32,
    pub pages_converted: u32,
    pub output_bytes: u64,
}

#[derive(Debug)]
pub enum HwpIngestError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse(String),
    Render(String),
    EmptyDocument,
    PageOutOfRange {
        requested: u32,
        page_count: u32,
    },
}

impl fmt::Display for HwpIngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "I/O error at {}: {source}", path.display())
            }
            Self::Parse(message) => write!(f, "failed to parse HWP document: {message}"),
            Self::Render(message) => write!(f, "failed to render PDF: {message}"),
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

impl Error for HwpIngestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse(_)
            | Self::Render(_)
            | Self::EmptyDocument
            | Self::PageOutOfRange { .. } => None,
        }
    }
}

impl From<RhwpAdapterError> for HwpIngestError {
    fn from(error: RhwpAdapterError) -> Self {
        match error {
            RhwpAdapterError::Parse(message) => Self::Parse(message),
            RhwpAdapterError::Render(message) => Self::Render(message),
            RhwpAdapterError::EmptyDocument => Self::EmptyDocument,
            RhwpAdapterError::PageOutOfRange {
                requested,
                page_count,
            } => Self::PageOutOfRange {
                requested,
                page_count,
            },
        }
    }
}

pub fn analyze_hwp_bytes(data: &[u8]) -> Result<DocumentInfo, HwpIngestError> {
    let info = hwp_ingest_rhwp_adapter::analyze_hwp_bytes(data)?;

    Ok(DocumentInfo {
        page_count: info.page_count,
    })
}

pub fn hwp_to_pdf_bytes(data: &[u8], options: ConvertOptions) -> Result<Vec<u8>, HwpIngestError> {
    hwp_ingest_rhwp_adapter::hwp_to_pdf_bytes(data, options.page_index).map_err(Into::into)
}

pub fn hwp_file_to_pdf_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: ConvertOptions,
) -> Result<ConvertReport, HwpIngestError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let data = std::fs::read(input_path).map_err(|source| HwpIngestError::Io {
        path: input_path.to_path_buf(),
        source,
    })?;

    let info = analyze_hwp_bytes(&data)?;
    let pdf = hwp_to_pdf_bytes(&data, options)?;

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| HwpIngestError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    std::fs::write(output_path, &pdf).map_err(|source| HwpIngestError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;

    Ok(ConvertReport {
        output_path: output_path.to_path_buf(),
        page_count: info.page_count,
        pages_converted: options.page_index.map_or(info.page_count, |_| 1),
        output_bytes: pdf.len() as u64,
    })
}

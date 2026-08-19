use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use hwp_ingest_rhwp_adapter::RhwpAdapterError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConvertOptions {
    pub page_index: Option<u32>,
    pub omit_header_footer: bool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgPage {
    pub page_index: u32,
    pub svg: Vec<u8>,
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
            Self::Render(message) => write!(f, "failed to render document: {message}"),
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
            RhwpAdapterError::PdfBackendUnavailable(message) => Self::Render(message),
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

fn adapter_render_options(options: ConvertOptions) -> hwp_ingest_rhwp_adapter::RenderOptions {
    hwp_ingest_rhwp_adapter::RenderOptions {
        page_index: options.page_index,
        omit_header_footer: options.omit_header_footer,
    }
}

pub fn analyze_hwp_bytes(data: &[u8]) -> Result<DocumentInfo, HwpIngestError> {
    let info = hwp_ingest_rhwp_adapter::analyze_hwp_bytes(data)?;

    Ok(DocumentInfo {
        page_count: info.page_count,
    })
}

pub fn hwp_to_pdf_bytes(data: &[u8], options: ConvertOptions) -> Result<Vec<u8>, HwpIngestError> {
    hwp_ingest_rhwp_adapter::hwp_to_pdf_bytes(data, adapter_render_options(options))
        .map_err(Into::into)
}

pub fn hwp_to_svg_pages(
    data: &[u8],
    options: ConvertOptions,
) -> Result<Vec<SvgPage>, HwpIngestError> {
    let pages = hwp_ingest_rhwp_adapter::hwp_to_svg_pages(data, adapter_render_options(options))?;

    Ok(pages
        .into_iter()
        .map(|page| SvgPage {
            page_index: page.page_index,
            svg: page.svg.into_bytes(),
        })
        .collect())
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

pub fn hwp_file_to_svg_files(
    input_path: impl AsRef<Path>,
    output_dir: Option<&Path>,
    options: ConvertOptions,
    overwrite: bool,
) -> Result<Vec<PathBuf>, HwpIngestError> {
    let input_path = input_path.as_ref();
    let data = std::fs::read(input_path).map_err(|source| HwpIngestError::Io {
        path: input_path.to_path_buf(),
        source,
    })?;

    let pages = hwp_to_svg_pages(&data, options)?;
    let target_dir = output_dir
        .map(Path::to_path_buf)
        .or_else(|| {
            input_path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| PathBuf::from("."));

    std::fs::create_dir_all(&target_dir).map_err(|source| HwpIngestError::Io {
        path: target_dir.clone(),
        source,
    })?;

    let stem = input_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"))
        .to_string_lossy();
    let planned_paths: Vec<PathBuf> = pages
        .iter()
        .map(|page| target_dir.join(format!("{stem}.page-{:04}.svg", page.page_index + 1)))
        .collect();

    if !overwrite {
        if let Some(existing_path) = planned_paths.iter().find(|path| path.exists()) {
            return Err(HwpIngestError::Io {
                path: existing_path.clone(),
                source: std::io::Error::new(
                    ErrorKind::AlreadyExists,
                    "output SVG file already exists",
                ),
            });
        }
    }

    for (page, path) in pages.iter().zip(planned_paths.iter()) {
        std::fs::write(path, &page.svg).map_err(|source| HwpIngestError::Io {
            path: path.clone(),
            source,
        })?;
    }

    Ok(planned_paths)
}

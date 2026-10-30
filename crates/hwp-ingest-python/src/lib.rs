use std::io::ErrorKind;
use std::path::Path;

use hwp_ingest::{
    self as core, ConvertOptions, DocumentStatus, HwpIngestError as CoreError, PageSelection,
    PageStatus, TextOptions,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyFileExistsError, PyFileNotFoundError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyModule, PyTuple};

create_exception!(hwp_ingest, HwpIngestError, pyo3::exceptions::PyException);
create_exception!(hwp_ingest, InvalidPageRangeError, PyValueError);

/// Basic metadata for an analyzed HWP document.
#[pyclass(frozen, module = "hwp_ingest._native")]
struct DocumentInfo {
    #[pyo3(get)]
    page_count: u32,
}

/// Report returned by native file-to-file PDF conversion.
#[pyclass(frozen, module = "hwp_ingest._native")]
struct ConvertReport {
    #[pyo3(get)]
    output_path: String,
    #[pyo3(get)]
    page_count: u32,
    #[pyo3(get)]
    pages_converted: u32,
    #[pyo3(get)]
    output_bytes: u64,
}

/// Text and status of one page in a text extraction result.
#[pyclass(frozen, module = "hwp_ingest._native")]
struct PageText {
    #[pyo3(get)]
    page_index: u32,
    #[pyo3(get)]
    status: String,
    #[pyo3(get)]
    status_detail: Option<String>,
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    artifact_key: String,
}

#[pymethods]
impl PageText {
    fn __repr__(&self) -> String {
        format!(
            "PageText(page_index={}, status={:?}, chars={})",
            self.page_index,
            self.status,
            self.text.chars().count()
        )
    }
}

/// Result of a page-range text extraction.
#[pyclass(frozen, module = "hwp_ingest._native")]
struct TextExtraction {
    #[pyo3(get)]
    document_status: String,
    #[pyo3(get)]
    document_status_detail: Option<String>,
    #[pyo3(get)]
    source_size_bytes: u64,
    #[pyo3(get)]
    source_sha256: String,
    #[pyo3(get)]
    source_format: Option<String>,
    #[pyo3(get)]
    source_file_name: Option<String>,
    #[pyo3(get)]
    page_count: Option<u32>,
    #[pyo3(get)]
    pagination: String,
    #[pyo3(get)]
    requested_pages: Option<(u32, u32)>,
    #[pyo3(get)]
    pages: Py<PyTuple>,
    #[pyo3(get)]
    extraction_version: u32,
    #[pyo3(get)]
    engine_version: String,
    #[pyo3(get)]
    rhwp_version: String,
}

#[pymethods]
impl TextExtraction {
    #[getter]
    fn is_ready(&self) -> bool {
        self.document_status == DocumentStatus::Ready.reason_code()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let page_count = self
            .page_count
            .map_or_else(|| "None".to_string(), |count| count.to_string());
        format!(
            "TextExtraction(document_status={:?}, page_count={page_count}, pages={})",
            self.document_status,
            self.pages.bind(py).len()
        )
    }
}

fn convert_options(page_index: Option<u32>, omit_header_footer: bool) -> ConvertOptions {
    ConvertOptions {
        page_index,
        omit_header_footer,
    }
}

fn core_error_to_py(error: CoreError) -> PyErr {
    match &error {
        CoreError::Io { source, .. } if source.kind() == ErrorKind::NotFound => {
            PyFileNotFoundError::new_err(error.to_string())
        }
        CoreError::Io { source, .. } if source.kind() == ErrorKind::AlreadyExists => {
            PyFileExistsError::new_err(error.to_string())
        }
        CoreError::PageOutOfRange { .. } | CoreError::EmptyDocument => {
            PyValueError::new_err(error.to_string())
        }
        CoreError::InvalidPageRange { .. } => InvalidPageRangeError::new_err(error.to_string()),
        CoreError::Io { .. } | CoreError::Parse(_) | CoreError::Render(_) => {
            HwpIngestError::new_err(error.to_string())
        }
    }
}

/// Analyze HWP bytes and return document metadata.
#[pyfunction]
#[pyo3(signature = (data))]
fn analyze_hwp_bytes(data: &[u8]) -> PyResult<DocumentInfo> {
    let info = core::analyze_hwp_bytes(data).map_err(core_error_to_py)?;

    Ok(DocumentInfo {
        page_count: info.page_count,
    })
}

/// Convert HWP bytes to PDF bytes.
#[pyfunction]
#[pyo3(signature = (data, page_index=None, omit_header_footer=false))]
fn hwp_to_pdf_bytes(
    py: Python<'_>,
    data: &[u8],
    page_index: Option<u32>,
    omit_header_footer: bool,
) -> PyResult<Py<PyBytes>> {
    let pdf = core::hwp_to_pdf_bytes(data, convert_options(page_index, omit_header_footer))
        .map_err(core_error_to_py)?;

    Ok(PyBytes::new(py, &pdf).unbind())
}

/// Render HWP bytes to SVG page bytes.
#[pyfunction]
#[pyo3(signature = (data, page_index=None, omit_header_footer=false))]
fn hwp_to_svg_pages(
    py: Python<'_>,
    data: &[u8],
    page_index: Option<u32>,
    omit_header_footer: bool,
) -> PyResult<Py<PyList>> {
    let pages = core::hwp_to_svg_pages(data, convert_options(page_index, omit_header_footer))
        .map_err(core_error_to_py)?;
    let list = PyList::empty(py);

    for page in pages {
        list.append(PyBytes::new(py, &page.svg))?;
    }

    Ok(list.unbind())
}

/// Convert an HWP file to a PDF file and return a conversion report.
#[pyfunction]
#[pyo3(signature = (input_path, output_path, page_index=None, omit_header_footer=false))]
fn hwp_to_pdf_file(
    input_path: &str,
    output_path: &str,
    page_index: Option<u32>,
    omit_header_footer: bool,
) -> PyResult<ConvertReport> {
    let report = core::hwp_file_to_pdf_file(
        input_path,
        output_path,
        convert_options(page_index, omit_header_footer),
    )
    .map_err(core_error_to_py)?;

    Ok(ConvertReport {
        output_path: report.output_path.to_string_lossy().into_owned(),
        page_count: report.page_count,
        pages_converted: report.pages_converted,
        output_bytes: report.output_bytes,
    })
}

/// Render an HWP file to SVG files and return created paths.
#[pyfunction]
#[pyo3(signature = (input_path, output_dir=None, page_index=None, overwrite=false, omit_header_footer=false))]
fn hwp_to_svg_files(
    input_path: &str,
    output_dir: Option<&str>,
    page_index: Option<u32>,
    overwrite: bool,
    omit_header_footer: bool,
) -> PyResult<Vec<String>> {
    let output_dir = output_dir.map(Path::new);
    let paths = core::hwp_file_to_svg_files(
        input_path,
        output_dir,
        convert_options(page_index, omit_header_footer),
        overwrite,
    )
    .map_err(core_error_to_py)?;

    Ok(paths
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

fn text_options(pages: Option<(u32, u32)>, omit_header_footer: bool) -> TextOptions {
    TextOptions {
        pages: match pages {
            Some((start, end)) => PageSelection::Range { start, end },
            None => PageSelection::All,
        },
        omit_header_footer,
    }
}

fn document_status_detail(status: &DocumentStatus) -> Option<String> {
    match status {
        DocumentStatus::Unparsable { detail } => Some(detail.clone()),
        DocumentStatus::TooLarge {
            size_bytes,
            limit_bytes,
        } => Some(format!(
            "input is {size_bytes} bytes, the limit is {limit_bytes} bytes"
        )),
        _ => None,
    }
}

fn text_extraction_to_py(py: Python<'_>, result: core::TextExtraction) -> PyResult<TextExtraction> {
    let mut pages = Vec::with_capacity(result.pages.len());
    for page in result.pages {
        let status_detail = match &page.status {
            PageStatus::ExtractionFailed { detail } => Some(detail.clone()),
            _ => None,
        };
        pages.push(Py::new(
            py,
            PageText {
                page_index: page.page_index,
                status: page.status.code().to_string(),
                status_detail,
                text: page.text,
                artifact_key: page.artifact_key,
            },
        )?);
    }

    Ok(TextExtraction {
        document_status: result.document_status.reason_code().to_string(),
        document_status_detail: document_status_detail(&result.document_status),
        source_size_bytes: result.source.size_bytes,
        source_sha256: result.source.sha256,
        source_format: result
            .source
            .format
            .map(|format| format.as_str().to_string()),
        source_file_name: result.source.file_name,
        page_count: result.page_count,
        pagination: result.pagination.as_str().to_string(),
        requested_pages: match result.requested {
            PageSelection::All => None,
            PageSelection::Range { start, end } => Some((start, end)),
        },
        pages: PyTuple::new(py, pages)?.unbind(),
        extraction_version: result.provenance.extraction_version,
        engine_version: result.provenance.engine_version.to_string(),
        rhwp_version: result.provenance.rhwp_version.to_string(),
    })
}

/// Extract page-range text from HWP or HWPX bytes.
#[pyfunction]
#[pyo3(signature = (data, pages=None, omit_header_footer=false))]
fn extract_text_bytes(
    py: Python<'_>,
    data: &[u8],
    pages: Option<(u32, u32)>,
    omit_header_footer: bool,
) -> PyResult<TextExtraction> {
    let result = core::extract_text_bytes(data, &text_options(pages, omit_header_footer))
        .map_err(core_error_to_py)?;

    text_extraction_to_py(py, result)
}

/// Extract page-range text from an HWP or HWPX file.
#[pyfunction]
#[pyo3(signature = (input_path, pages=None, omit_header_footer=false))]
fn extract_text_file(
    py: Python<'_>,
    input_path: &str,
    pages: Option<(u32, u32)>,
    omit_header_footer: bool,
) -> PyResult<TextExtraction> {
    let result = core::extract_text_file(input_path, &text_options(pages, omit_header_footer))
        .map_err(core_error_to_py)?;

    text_extraction_to_py(py, result)
}

#[pymodule]
#[pyo3(name = "_native")]
fn hwp_ingest_python(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("HwpIngestError", py.get_type::<HwpIngestError>())?;
    m.add(
        "InvalidPageRangeError",
        py.get_type::<InvalidPageRangeError>(),
    )?;
    m.add_class::<DocumentInfo>()?;
    m.add_class::<ConvertReport>()?;
    m.add_class::<PageText>()?;
    m.add_class::<TextExtraction>()?;
    m.add_function(wrap_pyfunction!(analyze_hwp_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_pdf_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_svg_pages, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_pdf_file, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_svg_files, m)?)?;
    m.add_function(wrap_pyfunction!(extract_text_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(extract_text_file, m)?)?;

    Ok(())
}

use std::io::ErrorKind;

use hwp_ingest::{self as core, ConvertOptions, HwpIngestError as CoreError};
use pyo3::create_exception;
use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

create_exception!(hwp_ingest, HwpIngestError, pyo3::exceptions::PyException);

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

fn convert_options(page_index: Option<u32>) -> ConvertOptions {
    ConvertOptions { page_index }
}

fn core_error_to_py(error: CoreError) -> PyErr {
    match &error {
        CoreError::Io { source, .. } if source.kind() == ErrorKind::NotFound => {
            PyFileNotFoundError::new_err(error.to_string())
        }
        CoreError::PageOutOfRange { .. } | CoreError::EmptyDocument => {
            PyValueError::new_err(error.to_string())
        }
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
#[pyo3(signature = (data, page_index=None))]
fn hwp_to_pdf_bytes(py: Python<'_>, data: &[u8], page_index: Option<u32>) -> PyResult<Py<PyBytes>> {
    let pdf =
        core::hwp_to_pdf_bytes(data, convert_options(page_index)).map_err(core_error_to_py)?;

    Ok(PyBytes::new(py, &pdf).unbind())
}

/// Convert an HWP file to a PDF file and return a conversion report.
#[pyfunction]
#[pyo3(signature = (input_path, output_path, page_index=None))]
fn hwp_to_pdf_file(
    input_path: &str,
    output_path: &str,
    page_index: Option<u32>,
) -> PyResult<ConvertReport> {
    let report = core::hwp_file_to_pdf_file(input_path, output_path, convert_options(page_index))
        .map_err(core_error_to_py)?;

    Ok(ConvertReport {
        output_path: report.output_path.to_string_lossy().into_owned(),
        page_count: report.page_count,
        pages_converted: report.pages_converted,
        output_bytes: report.output_bytes,
    })
}

#[pymodule]
#[pyo3(name = "_native")]
fn hwp_ingest_python(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("HwpIngestError", py.get_type::<HwpIngestError>())?;
    m.add_class::<DocumentInfo>()?;
    m.add_class::<ConvertReport>()?;
    m.add_function(wrap_pyfunction!(analyze_hwp_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_pdf_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(hwp_to_pdf_file, m)?)?;

    Ok(())
}

use std::{
    env,
    ffi::OsStr,
    fs,
    io::ErrorKind,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::RhwpAdapterError;

const RSVG_CONVERT_MISSING_MESSAGE: &str = "rsvg-convert executable was not found. Install librsvg tools to enable the default PDF backend. The svg2pdf backend is not used as an automatic fallback because it is known to drop clipped table text in some HWP documents.";

pub(crate) enum PdfBackend {
    RsvgConvert,
}

struct TempWorkspace {
    path: PathBuf,
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn svgs_to_pdf(svg_pages: &[String]) -> Result<Vec<u8>, RhwpAdapterError> {
    svgs_to_pdf_with_backend(svg_pages, PdfBackend::RsvgConvert)
}

fn svgs_to_pdf_with_backend(
    svg_pages: &[String],
    backend: PdfBackend,
) -> Result<Vec<u8>, RhwpAdapterError> {
    match backend {
        PdfBackend::RsvgConvert => svgs_to_pdf_with_program(svg_pages, OsStr::new("rsvg-convert")),
    }
}

fn svgs_to_pdf_with_program(
    svg_pages: &[String],
    program: &OsStr,
) -> Result<Vec<u8>, RhwpAdapterError> {
    if svg_pages.is_empty() {
        return Err(RhwpAdapterError::Render(
            "PDF export requires at least one SVG page".to_string(),
        ));
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let workspace = TempWorkspace {
        path: env::temp_dir().join(format!("hwp-ingest-rsvg-{}-{nanos}", std::process::id())),
    };
    fs::create_dir(&workspace.path).map_err(|error| {
        RhwpAdapterError::Render(format!("failed to create rsvg-convert workspace: {error}"))
    })?;

    let mut svg_paths = Vec::with_capacity(svg_pages.len());
    for (index, svg_page) in svg_pages.iter().enumerate() {
        let svg_path = workspace.path.join(format!("page-{index:04}.svg"));
        fs::write(&svg_path, svg_page).map_err(|error| {
            RhwpAdapterError::Render(format!(
                "failed to write SVG page at {}: {error}",
                svg_path.display()
            ))
        })?;
        svg_paths.push(svg_path);
    }

    let output_path = workspace.path.join("output.pdf");
    let output = Command::new(program)
        .arg("-f")
        .arg("pdf")
        .arg("-o")
        .arg(&output_path)
        .args(&svg_paths)
        .output()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                RhwpAdapterError::PdfBackendUnavailable(RSVG_CONVERT_MISSING_MESSAGE.to_string())
            } else {
                RhwpAdapterError::Render(format!("failed to run rsvg-convert PDF backend: {error}"))
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        let stderr = if stderr.is_empty() {
            "<no stderr>"
        } else {
            stderr
        };
        return Err(RhwpAdapterError::Render(format!(
            "rsvg-convert PDF backend failed with status {}: {stderr}",
            output.status
        )));
    }

    let pdf = fs::read(&output_path).map_err(|error| {
        RhwpAdapterError::Render(format!(
            "rsvg-convert PDF backend did not produce a readable PDF at {}: {error}",
            output_path.display()
        ))
    })?;

    if !pdf.starts_with(b"%PDF-") {
        return Err(RhwpAdapterError::Render(
            "rsvg-convert PDF backend produced output that does not start with %PDF-".to_string(),
        ));
    }

    Ok(pdf)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    const MINIMAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><text x="1" y="8">x</text></svg>"#;

    #[test]
    fn missing_rsvg_convert_reports_known_broken_svg2pdf_message() {
        let result = svgs_to_pdf_with_program(
            &[MINIMAL_SVG.to_string()],
            OsStr::new("__hwp_ingest_missing_rsvg_convert__"),
        );

        match result {
            Err(RhwpAdapterError::PdfBackendUnavailable(message)) => {
                assert!(message.contains("rsvg-convert executable was not found"));
                assert!(message.contains("not used as an automatic fallback"));
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn empty_svg_pages_are_rejected_before_backend_spawn() {
        let result = svgs_to_pdf_with_program(&[], OsStr::new("__unused__"));

        match result {
            Err(RhwpAdapterError::Render(message)) => {
                assert_eq!(message, "PDF export requires at least one SVG page");
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}

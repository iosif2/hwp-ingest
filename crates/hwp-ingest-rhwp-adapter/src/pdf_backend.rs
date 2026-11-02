use std::{
    env,
    ffi::OsStr,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::RhwpAdapterError;

const RSVG_CONVERT_MISSING_MESSAGE: &str = "rsvg-convert executable was not found. Install librsvg tools to enable the default PDF backend. The svg2pdf backend is not used as an automatic fallback because it is known to drop clipped table text in some HWP documents.";

/// Last-resort Korean font, embedded so PDF output keeps its Hangul on systems with no Korean font.
/// Its origin, license, and coverage are recorded in `assets/fonts/README.md`.
const FALLBACK_FONT: &[u8] = include_bytes!("../assets/fonts/NotoSansKR-Regular.ttf");
const FALLBACK_FONT_FILE_NAME: &str = "NotoSansKR-Regular.ttf";

/// Main fontconfig files tried when the caller did not set `FONTCONFIG_FILE`.
const SYSTEM_FONTCONFIG_FILES: [&str; 3] = [
    "/etc/fonts/fonts.conf",
    "/opt/homebrew/etc/fonts/fonts.conf",
    "/usr/local/etc/fonts/fonts.conf",
];

/// Fonts the PDF backend may use besides the ones installed on the system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PdfFontOptions {
    /// Directories scanned for fonts. They are listed ahead of the embedded fallback font.
    pub font_dirs: Vec<PathBuf>,
}

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

pub(crate) fn svgs_to_pdf(
    svg_pages: &[String],
    fonts: &PdfFontOptions,
) -> Result<Vec<u8>, RhwpAdapterError> {
    svgs_to_pdf_with_backend(svg_pages, PdfBackend::RsvgConvert, fonts)
}

fn svgs_to_pdf_with_backend(
    svg_pages: &[String],
    backend: PdfBackend,
    fonts: &PdfFontOptions,
) -> Result<Vec<u8>, RhwpAdapterError> {
    match backend {
        PdfBackend::RsvgConvert => svgs_to_pdf_with_program(
            svg_pages,
            OsStr::new("rsvg-convert"),
            fonts,
            env::var_os("FONTCONFIG_FILE").as_deref(),
        ),
    }
}

/// Builds the fontconfig file that adds the caller's font directories and the embedded fallback font
/// on top of the system configuration, and returns its path.
///
/// Returns `None`, leaving the backend on the system's own font setup, when there is no system
/// fontconfig file to build on or when the workspace cannot be written. Replacing the main file on a
/// platform whose default configuration is not found here would drop the system fonts, which is
/// worse than skipping the fallback.
fn prepare_fontconfig(
    workspace: &Path,
    fonts: &PdfFontOptions,
    caller_config: Option<&OsStr>,
) -> Option<PathBuf> {
    let base_configs = base_config_files(caller_config);
    if base_configs.is_empty() {
        return None;
    }

    let fallback_dir = workspace.join("fallback-fonts");
    let cache_dir = workspace.join("fontconfig-cache");
    fs::create_dir(&fallback_dir).ok()?;
    fs::create_dir(&cache_dir).ok()?;
    fs::write(fallback_dir.join(FALLBACK_FONT_FILE_NAME), FALLBACK_FONT).ok()?;

    let config_path = workspace.join("fonts.conf");
    let config = font_config_xml(&base_configs, &fonts.font_dirs, &fallback_dir, &cache_dir);
    fs::write(&config_path, config).ok()?;

    Some(config_path)
}

/// The main configuration files the generated file builds on: the caller's `FONTCONFIG_FILE` if it
/// exists, otherwise the usual system locations that exist.
fn base_config_files(caller_config: Option<&OsStr>) -> Vec<PathBuf> {
    let candidates: Vec<PathBuf> = match caller_config {
        Some(path) => vec![PathBuf::from(path)],
        None => SYSTEM_FONTCONFIG_FILES.iter().map(PathBuf::from).collect(),
    };
    candidates
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

/// `<cachedir>` comes first on purpose: fontconfig writes new cache files to the first writable
/// cache directory, and the fallback directory has a different path on every conversion. Placed
/// after the system configuration, each conversion would leave a stale file in the user's own
/// fontconfig cache.
fn font_config_xml(
    base_configs: &[PathBuf],
    font_dirs: &[PathBuf],
    fallback_dir: &Path,
    cache_dir: &Path,
) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n<fontconfig>\n",
    );
    xml.push_str(&format!("  <cachedir>{}</cachedir>\n", xml_path(cache_dir)));
    for base in base_configs {
        xml.push_str(&format!(
            "  <include ignore_missing=\"yes\">{}</include>\n",
            xml_path(base)
        ));
    }
    for dir in font_dirs {
        xml.push_str(&format!("  <dir>{}</dir>\n", xml_path(dir)));
    }
    xml.push_str(&format!("  <dir>{}</dir>\n", xml_path(fallback_dir)));
    xml.push_str("</fontconfig>\n");
    xml
}

fn xml_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn svgs_to_pdf_with_program(
    svg_pages: &[String],
    program: &OsStr,
    fonts: &PdfFontOptions,
    caller_fontconfig: Option<&OsStr>,
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
    let mut command = Command::new(program);
    command
        .arg("-f")
        .arg("pdf")
        .arg("-o")
        .arg(&output_path)
        .args(&svg_paths);
    if let Some(fontconfig) = prepare_fontconfig(&workspace.path, fonts, caller_fontconfig) {
        // Only this child process sees the generated configuration.
        command.env("FONTCONFIG_FILE", fontconfig);
    }
    let output = command.output().map_err(|error| {
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
            &PdfFontOptions::default(),
            None,
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
        let result = svgs_to_pdf_with_program(
            &[],
            OsStr::new("__unused__"),
            &PdfFontOptions::default(),
            None,
        );

        match result {
            Err(RhwpAdapterError::Render(message)) => {
                assert_eq!(message, "PDF export requires at least one SVG page");
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
    const PINNED_FALLBACK_FONT_SHA256: &str =
        "6e06a7fe5d696ca719894a23f36bb2b1be8c816a5937cd4ad0f23ca67780dd74";

    const HANGUL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="60" viewBox="0 0 240 60"><text x="10" y="40" font-family="Batang, serif" font-size="24">한글 대체 폰트</text></svg>"#;

    fn scratch_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "hwp-ingest-fonts-test-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("scratch directory should be created");
        dir
    }

    fn rsvg_convert_available() -> bool {
        Command::new("rsvg-convert")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn embedded_fallback_font_is_the_pinned_noto_sans_kr() {
        use sha2::{Digest, Sha256};

        let digest: String = Sha256::digest(FALLBACK_FONT)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        assert_eq!(digest, PINNED_FALLBACK_FONT_SHA256);
        assert_eq!(FALLBACK_FONT.len(), 2_519_996);
    }

    #[test]
    fn font_config_puts_the_cache_first_and_the_fallback_font_last() {
        let xml = font_config_xml(
            &[PathBuf::from("/etc/fonts/fonts.conf")],
            &[PathBuf::from("/fonts/a"), PathBuf::from("/fonts/b")],
            Path::new("/work/fallback"),
            Path::new("/work/cache"),
        );

        let position = |needle: &str| {
            xml.find(needle)
                .unwrap_or_else(|| panic!("{needle} missing from {xml}"))
        };
        assert!(position("<cachedir>/work/cache</cachedir>") < position("<include"));
        assert!(position("/etc/fonts/fonts.conf") < position("<dir>/fonts/a</dir>"));
        assert!(position("<dir>/fonts/a</dir>") < position("<dir>/fonts/b</dir>"));
        assert!(position("<dir>/fonts/b</dir>") < position("<dir>/work/fallback</dir>"));
        assert!(xml.contains("ignore_missing"));
    }

    #[test]
    fn font_config_escapes_xml_in_paths() {
        let xml = font_config_xml(
            &[PathBuf::from("/etc/a&b.conf")],
            &[PathBuf::from("/fonts/<x>\"y\"'z'")],
            Path::new("/work/fallback"),
            Path::new("/work/cache"),
        );

        assert!(xml.contains("/etc/a&amp;b.conf"));
        assert!(xml.contains("/fonts/&lt;x&gt;&quot;y&quot;&apos;z&apos;"));
        assert!(!xml.contains("<x>"));
    }

    #[test]
    fn base_config_prefers_the_callers_fontconfig_file() {
        let dir = scratch_dir("base");
        let caller = dir.join("caller.conf");
        fs::write(&caller, "<fontconfig/>").expect("config should be written");

        assert_eq!(base_config_files(Some(caller.as_os_str())), vec![caller]);
        // A missing caller file is not trusted as a base configuration.
        assert!(base_config_files(Some(dir.join("missing.conf").as_os_str())).is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepare_fontconfig_writes_the_fallback_font_and_a_config_that_lists_it() {
        let dir = scratch_dir("prepare");
        let caller = dir.join("caller.conf");
        fs::write(&caller, "<fontconfig/>").expect("config should be written");
        let fonts = PdfFontOptions {
            font_dirs: vec![PathBuf::from("/opt/company-fonts")],
        };

        let config = prepare_fontconfig(&dir, &fonts, Some(caller.as_os_str()))
            .expect("fontconfig should be prepared");

        assert_eq!(config, dir.join("fonts.conf"));
        let written = fs::read(dir.join("fallback-fonts").join(FALLBACK_FONT_FILE_NAME))
            .expect("fallback font should be written");
        assert_eq!(written, FALLBACK_FONT);
        let xml = fs::read_to_string(&config).expect("config should be readable");
        assert!(xml.contains("/opt/company-fonts"));
        assert!(xml.contains(&*dir.join("fallback-fonts").to_string_lossy()));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepare_fontconfig_is_skipped_without_a_system_configuration() {
        let dir = scratch_dir("skip");

        let config = prepare_fontconfig(
            &dir,
            &PdfFontOptions::default(),
            Some(dir.join("missing.conf").as_os_str()),
        );

        // With no base configuration to extend, the system's own font setup stays in charge.
        assert_eq!(config, None);
        assert!(!dir.join("fonts.conf").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pdf_embeds_the_fallback_font_when_the_system_has_no_fonts() {
        if !rsvg_convert_available() {
            eprintln!("rsvg-convert is not installed; skipping");
            return;
        }
        let dir = scratch_dir("pdf");
        // A system configuration that lists no fonts at all: the fallback is the only font.
        let empty_system = dir.join("empty-system.conf");
        fs::write(&empty_system, "<fontconfig/>").expect("config should be written");

        let pdf = svgs_to_pdf_with_program(
            &[HANGUL_SVG.to_string()],
            OsStr::new("rsvg-convert"),
            &PdfFontOptions::default(),
            Some(empty_system.as_os_str()),
        )
        .expect("PDF should render with the fallback font");

        assert!(pdf.starts_with(b"%PDF-"));
        assert!(
            contains(&pdf, b"NotoSansKR"),
            "the PDF should embed the fallback Hangul font"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversion_leaves_no_files_in_the_users_fontconfig_cache() {
        if !rsvg_convert_available() {
            eprintln!("rsvg-convert is not installed; skipping");
            return;
        }
        let Some(home) = env::var_os("HOME") else {
            return;
        };
        let user_cache = PathBuf::from(home).join(".cache").join("fontconfig");
        let count = || fs::read_dir(&user_cache).map_or(0, |entries| entries.count());
        let Some(system_config) = base_config_files(None).into_iter().next() else {
            return;
        };

        // Run twice so a first-time cache fill for the system's own fonts cannot hide a leak.
        let render = || {
            svgs_to_pdf_with_program(
                &[HANGUL_SVG.to_string()],
                OsStr::new("rsvg-convert"),
                &PdfFontOptions::default(),
                Some(system_config.as_os_str()),
            )
            .expect("PDF should render")
        };
        render();
        let before = count();
        render();
        render();

        assert_eq!(count(), before, "stale cache files were left behind");
    }
}

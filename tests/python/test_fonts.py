import ast
import inspect
import shutil
from pathlib import Path

import pytest

import hwp_ingest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures"
PACKAGE = ROOT / "python" / "hwp_ingest"
FALLBACK_FONT_DIR = ROOT / "crates" / "hwp-ingest-rhwp-adapter" / "assets" / "fonts"

FOOTNOTE_HWP = FIXTURES / "footnote-01.hwp"

needs_rsvg = pytest.mark.skipif(
    shutil.which("rsvg-convert") is None,
    reason="PDF conversion needs rsvg-convert",
)


@pytest.fixture
def system_without_fonts(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Path:
    """Make the PDF backend see a system fontconfig that lists no fonts at all.

    The embedded fallback font is then the only font available, whatever this machine has
    installed.
    """
    config = tmp_path / "empty-system.conf"
    config.write_text("<fontconfig/>", encoding="utf-8")
    monkeypatch.setenv("FONTCONFIG_FILE", str(config))
    return config


@needs_rsvg
def test_pdf_embeds_the_fallback_hangul_font_when_the_system_has_no_fonts(
    system_without_fonts: Path,
) -> None:
    pdf = hwp_ingest.to_pdf_bytes(FOOTNOTE_HWP.read_bytes(), page_index=0)

    assert pdf.startswith(b"%PDF-")
    assert b"NotoSansKR" in pdf


@needs_rsvg
def test_pdf_file_embeds_the_fallback_hangul_font_too(
    system_without_fonts: Path, tmp_path: Path
) -> None:
    output = hwp_ingest.to_pdf_file(FOOTNOTE_HWP, tmp_path / "out.pdf", page_index=0)

    assert b"NotoSansKR" in output.read_bytes()


@needs_rsvg
def test_font_dirs_accepts_str_path_and_tuple(tmp_path: Path) -> None:
    data = FOOTNOTE_HWP.read_bytes()

    for font_dirs in (
        [str(FALLBACK_FONT_DIR)],
        [FALLBACK_FONT_DIR],
        (FALLBACK_FONT_DIR, str(tmp_path)),
        [],
        None,
    ):
        pdf = hwp_ingest.to_pdf_bytes(data, page_index=0, font_dirs=font_dirs)

        assert pdf.startswith(b"%PDF-")


@needs_rsvg
def test_font_dirs_is_accepted_by_the_file_api(tmp_path: Path) -> None:
    output = hwp_ingest.to_pdf_file(
        FOOTNOTE_HWP,
        tmp_path / "out.pdf",
        page_index=0,
        font_dirs=[FALLBACK_FONT_DIR],
    )

    assert output.read_bytes().startswith(b"%PDF-")


def test_font_dirs_must_be_a_sequence_not_a_single_path(tmp_path: Path) -> None:
    data = FOOTNOTE_HWP.read_bytes()

    # A bare `str` is a `Sequence[str]` for a type checker, so only the runtime check catches it
    # (iterating it would silently treat every character as a directory).
    with pytest.raises(TypeError):
        hwp_ingest.to_pdf_bytes(data, font_dirs=str(tmp_path))
    with pytest.raises(TypeError):
        hwp_ingest.to_pdf_bytes(data, font_dirs=tmp_path)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        hwp_ingest.to_pdf_file(FOOTNOTE_HWP, tmp_path / "x.pdf", font_dirs=str(tmp_path))


def test_missing_or_non_directory_font_dirs_fail_loudly(tmp_path: Path) -> None:
    data = FOOTNOTE_HWP.read_bytes()
    not_a_directory = tmp_path / "font.ttf"
    not_a_directory.write_bytes(b"")

    with pytest.raises(FileNotFoundError):
        hwp_ingest.to_pdf_bytes(data, font_dirs=[tmp_path / "missing"])
    with pytest.raises(NotADirectoryError):
        hwp_ingest.to_pdf_bytes(data, font_dirs=[not_a_directory])
    with pytest.raises(FileNotFoundError):
        hwp_ingest.to_pdf_file(FOOTNOTE_HWP, tmp_path / "x.pdf", font_dirs=[tmp_path / "missing"])
    assert not (tmp_path / "x.pdf").exists()


def test_svg_output_does_not_take_font_options() -> None:
    parameters = inspect.signature(hwp_ingest.to_svg_pages).parameters

    assert "font_dirs" not in parameters
    assert "font_dirs" not in inspect.signature(hwp_ingest.to_svg_files).parameters


def stub_parameters(path: Path) -> dict[str, list[str]]:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    result: dict[str, list[str]] = {}
    for node in tree.body:
        if isinstance(node, ast.FunctionDef):
            args = node.args
            result[node.name] = [a.arg for a in args.posonlyargs + args.args + args.kwonlyargs]
    return result


def test_stub_signatures_match_the_runtime_for_every_public_function() -> None:
    stubs = stub_parameters(PACKAGE / "__init__.pyi")
    checked: set[str] = set()

    for name in hwp_ingest.__all__:
        member = getattr(hwp_ingest, name)
        if not inspect.isfunction(member):
            continue
        runtime = list(inspect.signature(member).parameters)
        assert stubs[name] == runtime, name
        checked.add(name)

    # A new public function must be added here on purpose, so it cannot skip this check unnoticed.
    assert checked == {
        "analyze_bytes",
        "to_pdf_bytes",
        "to_pdf_file",
        "to_svg_pages",
        "to_svg_files",
        "extract_text_bytes",
        "extract_text_file",
    }

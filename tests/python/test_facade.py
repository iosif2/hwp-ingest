from pathlib import Path

import pytest

import hwp_ingest

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "atop-equation-01.hwp"


def test_to_pdf_bytes_returns_pdf_bytes() -> None:
    data = FIXTURE.read_bytes()

    pdf = hwp_ingest.to_pdf_bytes(data)

    assert isinstance(pdf, bytes)
    assert pdf.startswith(b"%PDF-")
    assert len(pdf) > 1000



def test_to_svg_pages_returns_svg_page_bytes() -> None:
    pages = hwp_ingest.to_svg_pages(FIXTURE.read_bytes())

    assert isinstance(pages, list)
    assert pages
    assert all(isinstance(page, bytes) for page in pages)
    assert pages[0].lstrip().startswith(b"<svg")


def test_to_svg_pages_single_page_matches_first_page() -> None:
    data = FIXTURE.read_bytes()

    assert hwp_ingest.to_svg_pages(data, page_index=0) == [hwp_ingest.to_svg_pages(data)[0]]

def test_to_pdf_file_writes_pdf(tmp_path: Path) -> None:
    output_path = tmp_path / "out.pdf"

    result = hwp_ingest.to_pdf_file(FIXTURE, output_path)

    assert result == output_path
    assert output_path.exists()
    assert output_path.read_bytes().startswith(b"%PDF-")


def test_to_pdf_file_refuses_overwrite(tmp_path: Path) -> None:
    output_path = tmp_path / "out.pdf"
    output_path.write_bytes(b"existing")

    with pytest.raises(FileExistsError):
        hwp_ingest.to_pdf_file(FIXTURE, output_path, overwrite=False)


def test_negative_page_index_is_python_value_error() -> None:
    data = FIXTURE.read_bytes()

    with pytest.raises(ValueError, match="^page_index must be zero-based and non-negative$"):
        hwp_ingest.to_pdf_bytes(data, page_index=-1)

    with pytest.raises(ValueError, match="^page_index must be zero-based and non-negative$"):
        hwp_ingest.to_svg_pages(data, page_index=-1)

    with pytest.raises(ValueError, match="^page_index must be zero-based and non-negative$"):
        hwp_ingest.to_svg_pages(data, page_index=-1, omit_header_footer=True)

    with pytest.raises(ValueError, match="^page_index must be zero-based and non-negative$"):
        hwp_ingest.to_svg_files(FIXTURE, Path("unused"), page_index=-1)



def test_to_svg_files_writes_stable_paths(tmp_path: Path) -> None:
    paths = hwp_ingest.to_svg_files(FIXTURE, tmp_path, page_index=0)

    assert paths == [tmp_path / "atop-equation-01.page-0001.svg"]
    assert paths[0].read_bytes().lstrip().startswith(b"<svg")


def test_to_svg_files_refuses_overwrite(tmp_path: Path) -> None:
    existing = tmp_path / "atop-equation-01.page-0001.svg"
    existing.write_bytes(b"existing")

    with pytest.raises(FileExistsError):
        hwp_ingest.to_svg_files(FIXTURE, tmp_path, page_index=0, overwrite=False)


def test_to_svg_pages_does_not_require_rsvg_convert(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("PATH", "")

    pages = hwp_ingest.to_svg_pages(FIXTURE.read_bytes(), page_index=0)

    assert len(pages) == 1
    assert pages[0].lstrip().startswith(b"<svg")


def test_to_svg_pages_accepts_omit_header_footer_without_rsvg_convert(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("PATH", "")

    pages = hwp_ingest.to_svg_pages(
        FIXTURE.read_bytes(),
        page_index=0,
        omit_header_footer=True,
    )

    assert len(pages) == 1
    assert pages[0].lstrip().startswith(b"<svg")


def test_to_pdf_bytes_accepts_omit_header_footer() -> None:
    pdf = hwp_ingest.to_pdf_bytes(FIXTURE.read_bytes(), omit_header_footer=True)

    assert isinstance(pdf, bytes)
    assert pdf.startswith(b"%PDF-")
    assert len(pdf) > 1000


def test_to_pdf_file_accepts_omit_header_footer(tmp_path: Path) -> None:
    output_path = tmp_path / "out.pdf"

    result = hwp_ingest.to_pdf_file(
        FIXTURE,
        output_path,
        omit_header_footer=True,
    )

    assert result == output_path
    assert output_path.read_bytes().startswith(b"%PDF-")


def test_to_svg_files_accepts_omit_header_footer(tmp_path: Path) -> None:
    paths = hwp_ingest.to_svg_files(
        FIXTURE,
        tmp_path,
        page_index=0,
        omit_header_footer=True,
    )

    assert paths == [tmp_path / "atop-equation-01.page-0001.svg"]
    assert paths[0].read_bytes().lstrip().startswith(b"<svg")

def test_invalid_hwp_raises_package_error() -> None:
    with pytest.raises(hwp_ingest.HwpIngestError):
        hwp_ingest.to_pdf_bytes(b"not hwp")

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


def test_invalid_hwp_raises_package_error() -> None:
    with pytest.raises(hwp_ingest.HwpIngestError):
        hwp_ingest.to_pdf_bytes(b"not hwp")

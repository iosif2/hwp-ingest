from os import PathLike
from pathlib import Path

from . import _native

DocumentInfo = _native.DocumentInfo
HwpIngestError = _native.HwpIngestError


def _validate_page_index(page_index: int | None) -> int | None:
    if page_index is not None and page_index < 0:
        raise ValueError("page_index must be zero-based and non-negative")

    return page_index


def analyze_bytes(data: bytes) -> DocumentInfo:
    """HWP 바이트의 문서 정보를 분석합니다.

    Args:
        data: HWP 문서 바이트.

    Returns:
        분석된 문서 정보.

    Raises:
        HwpIngestError: 문서 파싱에 실패한 경우.
    """
    return _native.analyze_hwp_bytes(data)


def to_pdf_bytes(data: bytes, *, page_index: int | None = None) -> bytes:
    """HWP 바이트를 PDF 바이트로 변환합니다.

    Args:
        data: HWP 문서 바이트.
        page_index: 변환할 0기반 페이지 번호. None이면 전체 문서를 변환합니다.

    Returns:
        PDF 문서 바이트.

    Raises:
        ValueError: page_index가 음수이거나 문서 범위를 벗어난 경우.
        HwpIngestError: 문서 파싱 또는 렌더링에 실패한 경우.
    """
    return _native.hwp_to_pdf_bytes(data, _validate_page_index(page_index))


def to_pdf_file(
    input_path: str | PathLike[str],
    output_path: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
) -> Path:
    """HWP 파일을 PDF 파일로 변환합니다.

    Args:
        input_path: 입력 HWP 파일 경로.
        output_path: 출력 PDF 파일 경로. None이면 입력 경로의 확장자를 .pdf로 바꿉니다.
        page_index: 변환할 0기반 페이지 번호. None이면 전체 문서를 변환합니다.
        overwrite: 기존 출력 파일 덮어쓰기 여부.

    Returns:
        생성된 PDF 파일 경로.

    Raises:
        FileExistsError: 출력 파일이 있고 overwrite가 False인 경우.
        ValueError: page_index가 음수이거나 문서 범위를 벗어난 경우.
        HwpIngestError: 파일 읽기, 문서 파싱, 렌더링, 쓰기에 실패한 경우.
    """
    input_file = Path(input_path)
    output_file = Path(output_path) if output_path is not None else input_file.with_suffix(".pdf")

    if output_file.exists() and not overwrite:
        raise FileExistsError(output_file)

    report = _native.hwp_to_pdf_file(
        str(input_file),
        str(output_file),
        _validate_page_index(page_index),
    )
    return Path(report.output_path)

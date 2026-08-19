"""Pythonic facade for the native hwp-ingest extension."""

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
    """Analyze basic metadata from HWP document bytes.

    Args:
        data: Complete HWP document bytes.

    Returns:
        A `DocumentInfo` object containing page-count metadata.

    Raises:
        HwpIngestError: Raised when the HWP parser fails.
    """
    return _native.analyze_hwp_bytes(data)


def to_pdf_bytes(
    data: bytes,
    *,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> bytes:
    """HWP 문서 bytes를 PDF bytes로 변환합니다.

    `page_index`는 zero-based입니다. 생략하면 전체 문서를 하나의 PDF로
    변환합니다. PDF 변환에는 runtime에 `rsvg-convert` 실행 파일이
    필요합니다. `omit_header_footer=True`이면 렌더링된 PDF 페이지에서
    HWP 머리말/꼬리말 content를 생략합니다. 기본값 `False`는 기존
    렌더링을 보존하며, 페이지 layout이나 `page_index` 의미는 바꾸지
    않습니다.

    Args:
        data: 전체 HWP 문서 bytes.
        page_index: 변환할 zero-based 페이지 번호입니다. `None`이면 전체 문서를 변환합니다.
        omit_header_footer: 렌더링된 페이지에서 HWP 머리말/꼬리말 content를 생략할지 여부입니다.

    Returns:
        `%PDF-`로 시작하는 PDF 문서 bytes입니다.

    Raises:
        ValueError: `page_index`가 음수이거나 문서 범위를 벗어날 때 발생합니다.
        HwpIngestError: 파싱, 렌더링, 또는 PDF backend 실행이 실패할 때 발생합니다.
    """
    return _native.hwp_to_pdf_bytes(
        data,
        _validate_page_index(page_index),
        omit_header_footer,
    )


def to_svg_pages(
    data: bytes,
    *,
    page_index: int | None = None,
    omit_header_footer: bool = False,
) -> list[bytes]:
    """HWP 문서 페이지를 SVG bytes 목록으로 렌더링합니다.

    SVG는 페이지 단위 rendered artifact입니다. PDF backend나
    `rsvg-convert` 실행 파일 없이 생성됩니다. `omit_header_footer=True`이면
    렌더링된 SVG 페이지에서 HWP 머리말/꼬리말 content를 생략합니다.
    기본값 `False`는 기존 렌더링을 보존하며, 페이지 layout이나
    `page_index` 의미는 바꾸지 않습니다.

    Args:
        data: 전체 HWP 문서 bytes.
        page_index: 렌더링할 zero-based 페이지 번호입니다. `None`이면 전체 문서를 렌더링합니다.
        omit_header_footer: 렌더링된 페이지에서 HWP 머리말/꼬리말 content를 생략할지 여부입니다.

    Returns:
        선택된 페이지 순서의 SVG bytes 목록입니다.

    Raises:
        ValueError: `page_index`가 음수이거나 문서 범위를 벗어날 때 발생합니다.
        HwpIngestError: 파싱 또는 렌더링이 실패할 때 발생합니다.
    """
    return _native.hwp_to_svg_pages(
        data,
        _validate_page_index(page_index),
        omit_header_footer,
    )


def to_pdf_file(
    input_path: str | PathLike[str],
    output_path: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
    omit_header_footer: bool = False,
) -> Path:
    """HWP 파일을 PDF 파일로 변환합니다.

    `output_path`가 생략되면 입력 경로의 suffix를 `.pdf`로 바꾼 경로에
    씁니다. 기존 파일은 기본적으로 보호되며, `overwrite=True`를 전달하면
    대체합니다. PDF 변환에는 runtime에 `rsvg-convert` 실행 파일이
    필요합니다. `omit_header_footer=True`이면 렌더링된 PDF 페이지에서
    HWP 머리말/꼬리말 content를 생략합니다. 기본값 `False`는 기존
    렌더링을 보존하며, 페이지 layout이나 `page_index` 의미는 바꾸지
    않습니다.

    Args:
        input_path: 입력 HWP 파일 경로입니다.
        output_path: 출력 PDF 파일 경로입니다. `None`이면 입력 suffix를 `.pdf`로 바꿉니다.
        page_index: 변환할 zero-based 페이지 번호입니다. `None`이면 전체 문서를 변환합니다.
        overwrite: 기존 출력 파일을 대체할지 여부입니다.
        omit_header_footer: 렌더링된 페이지에서 HWP 머리말/꼬리말 content를 생략할지 여부입니다.

    Returns:
        생성된 PDF 파일 경로입니다.

    Raises:
        FileExistsError: 출력 파일이 있고 `overwrite`가 `False`일 때 발생합니다.
        FileNotFoundError: 입력 파일이 없을 때 발생합니다.
        ValueError: `page_index`가 음수이거나 문서 범위를 벗어날 때 발생합니다.
        HwpIngestError: 파일 I/O, 파싱, 렌더링, PDF backend 실행, 또는 출력 쓰기가 실패할 때 발생합니다.
    """
    input_file = Path(input_path)
    output_file = Path(output_path) if output_path is not None else input_file.with_suffix(".pdf")

    if output_file.exists() and not overwrite:
        raise FileExistsError(output_file)

    report = _native.hwp_to_pdf_file(
        str(input_file),
        str(output_file),
        _validate_page_index(page_index),
        omit_header_footer,
    )
    return Path(report.output_path)


def to_svg_files(
    input_path: str | PathLike[str],
    output_dir: str | PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
    omit_header_footer: bool = False,
) -> list[Path]:
    """HWP 문서 페이지를 SVG 파일로 렌더링하고 생성 경로를 반환합니다.

    `output_dir`가 생략되면 입력 파일의 parent directory에 씁니다.
    파일명은 `<stem>.page-0001.svg` 형식이며, Rust core가 페이지별
    파일명 계획과 overwrite 충돌 검사를 소유합니다.
    `omit_header_footer=True`이면 렌더링된 SVG 페이지에서 HWP
    머리말/꼬리말 content를 생략합니다. 기본값 `False`는 기존 렌더링을
    보존하며, 페이지 layout이나 `page_index` 의미는 바꾸지 않습니다.

    Args:
        input_path: 입력 HWP 파일 경로입니다.
        output_dir: SVG 파일을 쓸 디렉터리입니다. `None`이면 입력 파일의 parent directory를 사용합니다.
        page_index: 렌더링할 zero-based 페이지 번호입니다. `None`이면 전체 문서를 렌더링합니다.
        overwrite: 기존 SVG 파일을 대체할지 여부입니다.
        omit_header_footer: 렌더링된 페이지에서 HWP 머리말/꼬리말 content를 생략할지 여부입니다.

    Returns:
        선택된 페이지 순서의 생성 SVG 파일 경로 목록입니다.

    Raises:
        FileExistsError: 생성될 SVG 파일이 이미 있고 `overwrite`가 `False`일 때 발생합니다.
        FileNotFoundError: 입력 파일이 없을 때 발생합니다.
        ValueError: `page_index`가 음수이거나 문서 범위를 벗어날 때 발생합니다.
        HwpIngestError: 파일 I/O, 파싱, 렌더링, 또는 출력 쓰기가 실패할 때 발생합니다.
    """
    input_file = Path(input_path)
    output_directory = Path(output_dir) if output_dir is not None else None
    paths = _native.hwp_to_svg_files(
        str(input_file),
        str(output_directory) if output_directory is not None else None,
        _validate_page_index(page_index),
        overwrite,
        omit_header_footer,
    )
    return [Path(path) for path in paths]

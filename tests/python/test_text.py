import ast
import importlib.metadata
from pathlib import Path

import pytest

import hwp_ingest
from hwp_ingest import (
    HwpIngestError,
    InvalidPageRangeError,
    PageText,
    TextExtraction,
    extract_text_bytes,
    extract_text_file,
)
from hwp_ingest import _native

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures"
PACKAGE = ROOT / "python" / "hwp_ingest"

FOOTNOTE_HWP = FIXTURES / "footnote-01.hwp"
FOOTNOTE_HWPX = FIXTURES / "footnote-01.hwpx"
HEADER_FOOTER_HWP = FIXTURES / "hwp3-sample19-hwp5.hwp"
HEADER_FOOTER_HWPX = FIXTURES / "hwp3-sample19-hwpx.hwpx"

FOOTNOTE_HWP_SHA256 = "5bbc8a8fd23415aad59dd91d1eb261050946d9c64635933fcd49609ec2cc94e5"
EMPTY_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"


def total_chars(result: TextExtraction) -> int:
    return sum(len(page.text) for page in result.pages)


def assert_contract(result: TextExtraction) -> None:
    if not result.is_ready:
        assert result.pages == ()
        assert result.page_count is None
    for page in result.pages:
        assert bool(page.text) == (page.status == "text")
        assert page.artifact_key == (
            f"{result.source_sha256}:p{page.page_index}:v{result.extraction_version}"
        )
    assert result.pagination == "computed"


def test_first_page_of_hwp_and_hwpx_by_bytes() -> None:
    for path, source_format in ((FOOTNOTE_HWP, "hwp5"), (FOOTNOTE_HWPX, "hwpx")):
        result = extract_text_bytes(path.read_bytes(), pages=range(0, 1))

        assert isinstance(result, TextExtraction)
        assert_contract(result)
        assert result.is_ready
        assert result.document_status == "READY"
        assert result.document_status_detail is None
        assert result.source_format == source_format
        assert result.source_file_name is None
        assert result.page_count == 6
        assert result.requested_pages == (0, 1)
        assert isinstance(result.pages, tuple)
        assert len(result.pages) == 1
        assert isinstance(result.pages[0], PageText)
        assert result.pages[0].page_index == 0
        assert result.pages[0].status == "text"
        assert result.pages[0].status_detail is None
        assert result.pages[0].text


def test_whole_document_is_the_default_and_formats_agree() -> None:
    hwp = extract_text_bytes(FOOTNOTE_HWP.read_bytes())
    hwpx = extract_text_bytes(FOOTNOTE_HWPX.read_bytes())

    assert hwp.requested_pages is None
    assert len(hwp.pages) == len(hwpx.pages) == 6
    assert total_chars(hwp) == total_chars(hwpx) == 2035
    assert [page.text for page in hwp.pages] == [page.text for page in hwpx.pages]


def test_inner_range_keeps_absolute_page_indexes() -> None:
    result = extract_text_bytes(FOOTNOTE_HWPX.read_bytes(), pages=range(1, 3))

    assert [page.page_index for page in result.pages] == [1, 2]
    assert all(page.status == "text" for page in result.pages)


def test_pages_past_the_end_are_out_of_range_results_not_errors() -> None:
    result = extract_text_bytes(FOOTNOTE_HWP.read_bytes(), pages=range(4, 8))

    assert_contract(result)
    assert [(page.page_index, page.status) for page in result.pages] == [
        (4, "text"),
        (5, "text"),
        (6, "out_of_range"),
        (7, "out_of_range"),
    ]
    assert result.pages[2].text == "" and result.pages[3].text == ""


def test_file_input_accepts_str_and_path_and_records_the_file_name() -> None:
    from_path = extract_text_file(FOOTNOTE_HWPX, pages=range(0, 2))
    from_str = extract_text_file(str(FOOTNOTE_HWPX), pages=range(0, 2))
    from_bytes = extract_text_bytes(FOOTNOTE_HWPX.read_bytes(), pages=range(0, 2))

    assert from_path.source_file_name == "footnote-01.hwpx"
    assert from_str.source_file_name == "footnote-01.hwpx"
    assert [page.text for page in from_path.pages] == [page.text for page in from_bytes.pages]
    assert from_path.source_sha256 == from_str.source_sha256 == from_bytes.source_sha256


def test_source_identity_and_provenance() -> None:
    data = FOOTNOTE_HWP.read_bytes()
    result = extract_text_bytes(data, pages=range(0, 1))

    assert result.source_sha256 == FOOTNOTE_HWP_SHA256
    assert result.source_size_bytes == len(data)
    assert result.pages[0].artifact_key == f"{FOOTNOTE_HWP_SHA256}:p0:v1"
    assert result.extraction_version == 1
    assert result.rhwp_version == "0.8.7"
    assert result.engine_version == importlib.metadata.version("hwp-ingest")


def test_omit_header_footer_removes_header_and_footer_text() -> None:
    for path in (HEADER_FOOTER_HWP, HEADER_FOOTER_HWPX):
        data = path.read_bytes()

        with_hf = extract_text_bytes(data)
        without_hf = extract_text_bytes(data, omit_header_footer=True)

        assert with_hf.page_count == without_hf.page_count == 2
        assert total_chars(with_hf) == 1012
        assert total_chars(without_hf) == 888


@pytest.mark.parametrize("pages", [range(0, 0), range(2, 2), range(2, 1), range(0, 4097)])
def test_invalid_ranges_raise_invalid_page_range_error(pages: range) -> None:
    with pytest.raises(InvalidPageRangeError) as raised:
        extract_text_bytes(FOOTNOTE_HWP.read_bytes(), pages=pages)

    assert isinstance(raised.value, ValueError)
    assert not isinstance(raised.value, HwpIngestError)
    assert f"[{pages.start}, {pages.stop})" in str(raised.value)


def test_invalid_range_is_reported_even_when_the_document_is_unusable() -> None:
    with pytest.raises(InvalidPageRangeError):
        extract_text_bytes(b"", pages=range(2, 1))


def test_widest_allowed_range_is_accepted() -> None:
    result = extract_text_bytes(FOOTNOTE_HWP.read_bytes(), pages=range(0, 4096))

    assert result.is_ready
    assert len(result.pages) == 4096
    assert [page.status for page in result.pages[:6]] == ["text"] * 6
    assert all(page.status == "out_of_range" for page in result.pages[6:])


def test_pages_must_be_a_unit_step_range() -> None:
    data = FOOTNOTE_HWP.read_bytes()

    with pytest.raises(TypeError):
        extract_text_bytes(data, pages=(0, 1))  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        extract_text_bytes(data, pages=1)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="step"):
        extract_text_bytes(data, pages=range(0, 4, 2))
    with pytest.raises(ValueError, match="non-negative"):
        extract_text_bytes(data, pages=range(-1, 2))
    with pytest.raises(ValueError, match="32 bits"):
        extract_text_bytes(data, pages=range(0, 2**32))


def test_unreadable_documents_are_statuses_without_any_text() -> None:
    hwp = FOOTNOTE_HWP.read_bytes()
    hml = b'<?xml version="1.0" encoding="UTF-8"?><HWPML Version="2.8"><BODY></BODY></HWPML>'
    cases = [
        ("empty", b"", "EMPTY_INPUT", None),
        ("garbage", b"this is not a document", "UNSUPPORTED_FORMAT", None),
        ("fasoo drm", b"\x9b DRMONE" + bytes(64), "DRM_PROTECTED", None),
        ("truncated hwp", hwp[: len(hwp) // 10], "UNPARSABLE", "hwp5"),
        ("hml", hml, "PAGE_BOUNDARY_UNSUPPORTED", "hml"),
    ]

    for label, data, code, source_format in cases:
        for pages in (None, range(0, 1)):
            result = extract_text_bytes(data, pages=pages)

            assert_contract(result)
            assert not result.is_ready, label
            assert result.document_status == code, label
            assert result.source_format == source_format, label
            assert result.source_size_bytes == len(data), label

    assert extract_text_bytes(b"").source_sha256 == EMPTY_SHA256
    assert extract_text_bytes(hwp[: len(hwp) // 10]).document_status_detail


def test_missing_file_is_a_file_not_found_error() -> None:
    with pytest.raises(FileNotFoundError):
        extract_text_file(FIXTURES / "does-not-exist.hwp")


def test_results_are_immutable_and_have_a_compact_repr() -> None:
    result = extract_text_bytes(FOOTNOTE_HWP.read_bytes(), pages=range(0, 1))

    with pytest.raises(AttributeError):
        result.page_count = 3  # type: ignore[misc]
    with pytest.raises(AttributeError):
        result.pages[0].text = "x"  # type: ignore[misc]
    assert repr(result) == 'TextExtraction(document_status="READY", page_count=6, pages=1)'
    assert "PageText(page_index=0" in repr(result.pages[0])
    assert result.pages[0].text not in repr(result.pages[0])
    assert repr(extract_text_bytes(b"")) == (
        'TextExtraction(document_status="EMPTY_INPUT", page_count=None, pages=0)'
    )


def test_conversion_api_is_unchanged() -> None:
    data = FOOTNOTE_HWP.read_bytes()

    assert hwp_ingest.analyze_bytes(data).page_count == 6
    assert len(hwp_ingest.to_svg_pages(data)) == 6


def stub_names(path: Path) -> set[str]:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    names: set[str] = set()
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
            names.add(node.name)
        elif isinstance(node, ast.ImportFrom):
            names.update(alias.asname or alias.name for alias in node.names)
        elif isinstance(node, ast.Assign):
            names.update(t.id for t in node.targets if isinstance(t, ast.Name))
    return names


def test_every_public_symbol_has_stub_coverage() -> None:
    package_stub = stub_names(PACKAGE / "__init__.pyi")
    native_stub = stub_names(PACKAGE / "_native.pyi")

    assert set(hwp_ingest.__all__) <= package_stub
    native_public = {name for name in dir(_native) if not name.startswith("_")}
    assert native_public <= native_stub, native_public - native_stub
    assert (PACKAGE / "py.typed").is_file()

    stub_tree = ast.parse((PACKAGE / "__init__.pyi").read_text(encoding="utf-8"))
    stub_all = next(
        ast.literal_eval(node.value)
        for node in stub_tree.body
        if isinstance(node, ast.Assign)
        and any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets)
    )
    assert sorted(stub_all) == sorted(hwp_ingest.__all__)

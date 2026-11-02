<h1 align="center">hwp-ingest</h1>

<p align="center">
  <a href="https://github.com/iosif2/hwp-ingest/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/iosif2/hwp-ingest/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://pypi.org/project/hwp-ingest/"><img alt="PyPI" src="https://img.shields.io/pypi/v/hwp-ingest.svg"></a>
  <a href="https://pypi.org/project/hwp-ingest/"><img alt="Python versions" src="https://img.shields.io/pypi/pyversions/hwp-ingest.svg"></a>
  <a href="https://github.com/iosif2/hwp-ingest/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/pypi/l/hwp-ingest.svg"></a>
</p>

<p align="center">
  <strong>Rust-backed Python package for Korean HWP/HWPX ingestion.</strong><br>
  Convert HWP documents to PDF or page-level SVG artifacts through a typed Python facade.
</p>

> `hwp-ingest` uses a vendored `rhwp` subset as its current HWP parsing,
> layout, and rendering backend. The vendored backend can carry hwp-ingest-local
> compatibility patches, while product-facing conversion policy stays in the
> hwp-ingest Rust/Python layers.

## Features

- Convert HWP files to PDF bytes or PDF files. When the system has no Korean font, Hangul falls back to a bundled open-licensed font instead of turning into empty boxes.
- Render page-level SVG artifacts for inspection and downstream pipelines.
- Read basic document metadata such as renderable page count.
- Extract the text of a page range, the first page included, from HWP and HWPX, with an explicit status for every page and for documents that cannot be read.
- Keep conversion orchestration in Rust while Python handles paths, overwrite policy, and typed facade ergonomics.

Install from PyPI:

```sh
uv pip install hwp-ingest
# or
python -m pip install hwp-ingest
```

## Runtime PDF dependency

PDF conversion requires the `rsvg-convert` executable from librsvg at runtime.
The Python wheel does not bundle this executable.
SVG output does not require rsvg-convert; that executable is only needed for PDF conversion.

Check availability:

```sh
rsvg-convert --version
```

Install examples:

```sh
# Debian/Ubuntu
sudo apt-get install librsvg2-bin

# Fedora/RHEL
sudo dnf install librsvg2-tools

# Arch Linux
sudo pacman -S librsvg

# macOS
brew install librsvg
```

On Windows, install librsvg/rsvg-convert and ensure `rsvg-convert.exe` is on
`PATH`. If `rsvg-convert` is missing, PDF conversion raises
`hwp_ingest.HwpIngestError` with a clear message. `svg2pdf` is not used as an
automatic fallback because it is known to drop clipped table text in some HWP
documents.

## Fallback font for Hangul

PDF conversion first uses the fonts installed on the system. If a document's
font is missing and the system has no Korean font at all, `hwp-ingest` falls
back to a bundled Noto Sans KR Regular subset (SIL Open Font License 1.1, see
`licenses/NotoSansKR-OFL.txt`) so Hangul is rendered instead of missing-glyph
boxes. Fonts you pass with `font_dirs` and the system fonts take precedence
over the bundled one.

```python
hwp_ingest.to_pdf_bytes(data, font_dirs=["/opt/company-fonts"])
```

The bundled font covers Hangul and Latin only. It does **not** cover Hanja,
circled digits such as ①, fullwidth symbols, most arrows and math symbols, or
Hancom private-use symbols, and it is a single regular-weight sans-serif face:
serif (Myeongjo/Batang) text is drawn in that sans-serif face and bold is
synthesized. For those characters
install a full CJK font on the machine or pass a directory with `font_dirs`:

```sh
# Debian/Ubuntu: full Korean/CJK coverage including Hanja
sudo apt-get install fonts-noto-cjk
```

SVG output cannot carry fonts, so whatever displays the SVG needs its own
Korean font. The fallback only applies to PDF output.

## Font fallback and PUA glyphs

`hwp-ingest` does not bundle Hancom or Microsoft fonts. PDF conversion asks
`rsvg-convert`/fontconfig to resolve the SVG `font-family` chain from fonts
installed on the system. If a document contains visible Private Use Area (PUA)
characters, such as Hancom symbol slots, a generic Unicode font is not enough:
PUA codepoints do not define a standard glyph shape. They render correctly only
when the owning document font is installed, or when the project adds a
source-backed codepoint mapping for that specific symbol set.

The current renderer preserves the document font name first, then appends
Korean-compatible fallback families:

| Text family in the document | Fallback order |
| --- | --- |
| Batang/Myeongjo/serif text | Document font → `Batang`/`바탕` → `Nanum Myeongjo` → `AppleMyungjo` → `Noto Serif KR` / `Noto Serif CJK KR` → `HCR Batang Ext-B` / `함초롬바탕 확장B` → `HCR Batang Ext` / `함초롬바탕 확장` → `HCR Batang` / `함초롬바탕` → `Source Han Serif K Old Hangul` → `serif` |
| Dotum/Gothic/Gulim/sans-serif text | Document font → `Malgun Gothic` / `맑은 고딕` → `Apple SD Gothic Neo` → `Noto Sans KR ExtraLight` → `Noto Sans KR` → `Pretendard` → `HCR Batang Ext-B` / `함초롬바탕 확장B` → `HCR Batang Ext` / `함초롬바탕 확장` → `HCR Batang` / `함초롬바탕` → `Source Han Serif K Old Hangul` → `sans-serif` |

For Hancom-origin documents with PUA symbols, install the matching licensed font
on the machine running conversion:

| Document/font origin | Families to check first |
| --- | --- |
| Hancom/HCR body or PUA symbols | `HCR Batang` / `함초롬바탕`, `HCR Dotum` / `함초롬돋움` |
| Windows-origin documents | `Batang`, `Dotum`, `Gulim`, `Gungsuh`, `Malgun Gothic` |
| Older HY/Hanyang documents | `HY울릉도L`, `HY헤드라인M`, `HY견고딕`, `HY그래픽`, `HY견명조`, `HY신명조` |

Linux/fontconfig check:

```sh
fc-match "HCR Batang"
fc-match "함초롬바탕"
fc-match "HCR Dotum"
```

If these commands resolve to a fallback like Noto or DejaVu instead of the
expected Hancom/HCR family, install the licensed TTF/OTF files into a system or
user font directory and refresh fontconfig:

```sh
# user-local example
mkdir -p ~/.local/share/fonts/hancom
cp HCRBatang.ttf HCRBatang-Bold.ttf ~/.local/share/fonts/hancom/
fc-cache -f ~/.local/share/fonts/hancom
```

Do not commit restricted font files into this repository. The only font
bundled is the redistributable Noto Sans KR subset described above; Hancom and
Microsoft fonts are never bundled.

## Quick start

Convert an HWP file to a PDF next to the input file:

```python
from pathlib import Path

import hwp_ingest

input_path = Path("document.hwp")
output_path = hwp_ingest.to_pdf_file(input_path, overwrite=True)
print(output_path)
```

Convert one zero-based page to bytes:

```python
from pathlib import Path

import hwp_ingest

pdf_bytes = hwp_ingest.to_pdf_bytes(
    Path("document.hwp").read_bytes(),
    page_index=0,
)
Path("page-1.pdf").write_bytes(pdf_bytes)
```

Render SVG page artifacts without the PDF backend:

```python
from pathlib import Path

import hwp_ingest

svg_pages = hwp_ingest.to_svg_pages(Path("document.hwp").read_bytes())
Path("page-1.svg").write_bytes(svg_pages[0])
```

Extract the text of the first page (zero-based, half-open page range):

```python
from pathlib import Path

import hwp_ingest

result = hwp_ingest.extract_text_file(
    Path("document.hwp"),
    pages=range(0, 1),
    omit_header_footer=True,
)
if result.is_ready and result.pages[0].status == "text":
    first_page_text = result.pages[0].text
else:
    # Not an error: continue with the file name or post title instead.
    print(result.document_status, [page.status for page in result.pages])
```

Read basic document metadata:

```python
from pathlib import Path

import hwp_ingest

info = hwp_ingest.analyze_bytes(Path("document.hwp").read_bytes())
print(info.page_count)
```

## Python API

The package is typed and ships `py.typed` plus checked-in `.pyi` stubs.

```python
hwp_ingest.analyze_bytes(data: bytes) -> hwp_ingest.DocumentInfo
```

Returns a `DocumentInfo` object. `DocumentInfo.page_count` is the number of
renderable pages discovered in the HWP bytes.

```python
hwp_ingest.to_pdf_bytes(
    data: bytes,
    *,
    page_index: int | None = None,
    font_dirs: Sequence[str | os.PathLike[str]] | None = None,
) -> bytes
```

Returns PDF bytes. `page_index` is zero-based. Pass `None` to convert the full
document. `font_dirs` lists extra directories to search for fonts (see
[Fallback font for Hangul](#fallback-font-for-hangul)); pass a sequence of
paths, not a single path.

```python
hwp_ingest.to_svg_pages(
    data: bytes,
    *,
    page_index: int | None = None,
) -> list[bytes]
```

Returns one SVG document per selected page as bytes. `page_index` is
zero-based. Pass `None` to render all pages in document order. SVG output does
not require `rsvg-convert`.

```python
hwp_ingest.to_pdf_file(
    input_path: str | os.PathLike[str],
    output_path: str | os.PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
    font_dirs: Sequence[str | os.PathLike[str]] | None = None,
) -> pathlib.Path
```

Writes a PDF and returns the output path. If `output_path` is `None`, the output
uses the input path with a `.pdf` suffix. Existing output files are protected
unless `overwrite=True`.

```python
hwp_ingest.to_svg_files(
    input_path: str | os.PathLike[str],
    output_dir: str | os.PathLike[str] | None = None,
    *,
    page_index: int | None = None,
    overwrite: bool = False,
) -> list[pathlib.Path]
```

Writes one SVG file per selected page and returns paths in page order. If
`output_dir` is `None`, SVG files are written beside the input file. Generated
filenames are stable and use one-based visible page numbers:
`<stem>.page-0001.svg`, `<stem>.page-0002.svg`, and so on. Existing output
files are protected unless `overwrite=True`.

```python
hwp_ingest.extract_text_bytes(
    data: bytes,
    *,
    pages: range | None = None,
    omit_header_footer: bool = False,
) -> hwp_ingest.TextExtraction

hwp_ingest.extract_text_file(
    input_path: str | os.PathLike[str],
    *,
    pages: range | None = None,
    omit_header_footer: bool = False,
) -> hwp_ingest.TextExtraction
```

Extracts page text from HWP 5.0, HWPX, and HWP 3.0 documents. `pages` is a
zero-based, half-open `range`: `range(0, 1)` is the first page, `range(1, 3)`
the second and third. Pass `None` for the whole document. A range may span at
most 4096 pages. `omit_header_footer=True` leaves header and footer text out of
every page.

A document that cannot be read is **not** an exception. The result says why in
`document_status`, and then `pages` is empty and no text of any kind is
returned: there is never a document excerpt standing in for a page.

| `document_status` | Meaning |
| --- | --- |
| `READY` | The document opened and its pages were computed. |
| `UNSUPPORTED_FORMAT` | Not a document format hwp-ingest reads. |
| `DRM_PROTECTED` | Protected by a DRM container. |
| `PASSWORD_REQUIRED` | Needs a password (opening with a password is not supported). |
| `EMPTY_INPUT` | Zero bytes. |
| `UNPARSABLE` | A known format that could not be read; see `document_status_detail`. |
| `TOO_LARGE` | Input over the size limit (256 MB). |
| `PAGE_BOUNDARY_UNSUPPORTED` | HML opens but its page boundaries are unverified, so no text is returned. |
| `NO_PAGES` | The document opened with zero pages. |

Each entry of `result.pages` has a `page_index`, a `status`, the `text`, and a
stable `artifact_key`. Pages past the end of the document stay in the list as
`out_of_range` instead of failing the request.

| `status` | Meaning |
| --- | --- |
| `text` | The page has text. |
| `no_text` | The page was read and has no text, for example an image-only page. This is common and not a failure. |
| `extraction_failed` | Extracting this page failed; see `status_detail`. |
| `out_of_range` | The page is past the end of the document. |

The result also records where it came from: `source_sha256`, `source_size_bytes`,
`source_format`, `source_file_name` (file API only), `page_count`, and the
`extraction_version`, `engine_version`, and `rhwp_version` that produced it.
`pagination` is always `"computed"`: HWP and HWPX do not store page boundaries,
so pages are computed by the layout engine and can differ from the pages
Hancom Office shows, mostly by a page or so late in long documents. Treat page
numbers as an estimate, not a guarantee.

What to expect from the text itself: equations come out as their equation
script, circled digits such as ① stay as they are, table text is included, and
object placeholders (`U+FFFC`) are removed. Header and footer text is included
unless you pass `omit_header_footer=True`. A damaged file can still open
without any error, so a `READY` result does not prove the file was intact.

### Errors

- `ValueError`: negative `page_index`, empty documents, or out-of-range pages.
  `hwp_ingest.InvalidPageRangeError` (a `ValueError`): the text extraction range
  is empty, inverted, or wider than 4096 pages. Other `pages` mistakes raise
  `TypeError` (not a `range`) or `ValueError` (step other than 1, negative).
- `FileExistsError`: output exists and `overwrite=False`.
- `FileNotFoundError`: input file path does not exist, or a `font_dirs` entry
  does not exist. `NotADirectoryError`: a `font_dirs` entry is not a directory.
  `TypeError`: `font_dirs` is a single path instead of a sequence.
- `hwp_ingest.HwpIngestError`: parse, render, PDF backend, or output write
  failures.

## Local development

```sh
cargo test --workspace
uv run maturin develop
uv run pytest
```

Build distributable artifacts locally:

```sh
uv run maturin sdist --out dist
uv run --with ziglang maturin build --release --locked --out dist --compatibility pypi --zig
```

## Notices

This project includes vendored portions of `rhwp`.

본 제품은 한글과컴퓨터의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

"한글", "한컴", "HWP", and "HWPX" are registered trademarks of Hancom Inc.
`hwp-ingest` is an independent open-source project and is not affiliated with,
sponsored by, or endorsed by Hancom Inc.

See `NOTICE`, `THIRD_PARTY_LICENSES.md`, and `vendor/rhwp-subset/NOTICE` for
vendored source attribution and license details.

## Project direction

See [docs/rhwp-python-binding-plan.md](docs/rhwp-python-binding-plan.md) for
batch engine, semantic output, vendor/upstream sync, and production pipeline
direction.

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

- Convert HWP files to PDF bytes or PDF files.
- Render page-level SVG artifacts for inspection and downstream pipelines.
- Read basic document metadata such as renderable page count.
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

Do not commit restricted font files into this repository.

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
) -> bytes
```

Returns PDF bytes. `page_index` is zero-based. Pass `None` to convert the full
document.

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

### Errors

- `ValueError`: negative `page_index`, empty documents, or out-of-range pages.
- `FileExistsError`: output exists and `overwrite=False`.
- `FileNotFoundError`: input file path does not exist.
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

<h1 align="center">hwp-ingest</h1>

<p align="center">
  <a href="https://github.com/iosif2/hwp-ingest/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/iosif2/hwp-ingest/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://pypi.org/project/hwp-ingest/"><img alt="PyPI" src="https://img.shields.io/pypi/v/hwp-ingest.svg"></a>
  <a href="https://pypi.org/project/hwp-ingest/"><img alt="Python versions" src="https://img.shields.io/pypi/pyversions/hwp-ingest.svg"></a>
  <a href="https://github.com/iosif2/hwp-ingest/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/pypi/l/hwp-ingest.svg"></a>
</p>

<p align="center">
  <strong>Rust-backed Python package for Korean HWP/HWPX ingestion.</strong><br>
  Current public alpha: HWP to PDF conversion through a small typed Python facade.
</p>

## Current alpha scope

`0.1.0a1` is a public alpha for HWP to PDF conversion. Install it with an exact
pre-release pin:

```sh
uv pip install hwp-ingest==0.1.0a1
# or
python -m pip install hwp-ingest==0.1.0a1
```

Release CI is configured to build Linux x86_64/aarch64, macOS Intel/Apple
Silicon, and Windows x86_64 wheels. Native runner wheel jobs install
`rsvg-convert` and run a PDF conversion smoke test; the Linux aarch64 job
cross-builds the wheel and verifies the artifact payload before publishing.

## Runtime PDF dependency

PDF conversion requires the `rsvg-convert` executable from librsvg at runtime.
The Python wheel does not bundle this executable.

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

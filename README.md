# hwp-ingest

`hwp-ingest` is a Rust/Python HWP/HWPX ingestion toolkit for native conversion, rendered artifacts, and semantic document outputs.
The current MVP keeps a small Pythonic facade around Rust-native analysis and PDF conversion while the Rust core remains the reusable boundary for future batch and semantic pipelines.

## Runtime PDF dependency

PDF conversion uses `rsvg-convert` from librsvg at runtime. The Rust and Python
packages do not bundle this executable, so install it before calling
`hwp_ingest.to_pdf_bytes` or `hwp_ingest.to_pdf_file`.

Check availability:

```sh
rsvg-convert --version
```

Common install commands:

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

On Windows, install librsvg for Windows and ensure `rsvg-convert.exe` is on
`PATH`. If `rsvg-convert` is missing, PDF conversion fails with a clear
`hwp_ingest.HwpIngestError`; `svg2pdf` is not used as an automatic fallback
because it is known to drop clipped table text in some HWP documents.

## Local development

```sh
cargo test --workspace
uv run maturin develop
uv run pytest
```

Build distributable artifacts locally:

```sh
uv run maturin build --release --locked --out dist --compatibility pypi
uv run maturin sdist --out dist
```

## Python example

```python
from pathlib import Path

import hwp_ingest

input_path = Path("document.hwp")
output_path = hwp_ingest.to_pdf_file(input_path, overwrite=True)
print(output_path)
```

See [docs/rhwp-python-binding-plan.md](docs/rhwp-python-binding-plan.md) for batch engine, semantic output, vendor/upstream sync, and production pipeline direction.
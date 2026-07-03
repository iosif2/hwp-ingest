# hwp-ingest

`hwp-ingest` is a Rust/Python HWP/HWPX ingestion toolkit for native conversion, rendered artifacts, and semantic document outputs.
The current MVP keeps a small Pythonic facade around Rust-native analysis and PDF conversion while the Rust core remains the reusable boundary for future batch and semantic pipelines.

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
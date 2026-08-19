# SVG artifact API decision

## Status

Accepted.

## Context

`hwp-ingest`의 현재 PDF path는 HWP 문서를 page별 SVG render output으로 만든 뒤 PDF backend로 변환한다. PDF 결과가 깨질 때 원인이 HWP parsing/layout/SVG rendering 단계인지, SVG-to-PDF backend 단계인지 분리해서 확인할 수 있어야 한다.

SVG는 이 문제를 진단하는 데 유용하지만, API 관점에서는 debug dump가 아니다. SVG는 PDF와 같은 rendered artifact 범주의 정상적인 변환 산출물이다.

## Decision

SVG output은 first-class rendered artifact API로 설계한다. Public API 이름에는 `debug` prefix를 붙이지 않는다.

Python API direction:

```python
to_svg_pages(data: bytes, *, page_index: int | None = None, omit_header_footer: bool = False) -> list[bytes]
to_svg_files(input_path, output_dir=None, *, page_index=None, overwrite=False, omit_header_footer=False) -> list[Path]
```

Rust engine API direction:

```rust
hwp_to_svg_pages(data: &[u8], options: ConvertOptions) -> Result<Vec<SvgPage>, HwpIngestError>
hwp_file_to_svg_files(input_path, output_dir, options) -> Result<Vec<PathBuf>, HwpIngestError>
```

The concrete Rust type names may change during implementation, but the boundary does not change: `hwp-ingest` owns the public API shape and conversion policy, `hwp-ingest-rhwp-adapter` absorbs `rhwp` render output, and Python only exposes Pythonic path and bytes behavior.

## API semantics

`to_svg_pages` returns one SVG document per selected page as bytes. It always returns `list[bytes]`, including when `page_index` selects a single page. This avoids a return type that changes between `bytes` and `list[bytes]` depending on the option.

`page_index` follows the existing MVP convention: it is a 0-based page index, and `None` means all pages in document order.

`omit_header_footer`는 SVG와 PDF가 공유하는 render option이다. hwp-ingest PDF output은 SVG-derived이므로, 이 옵션은 SVG render 단계에서 적용되고 PDF는 동일한 page artifact를 사용한다.

`to_svg_files` writes one `.svg` file per selected page and returns the written paths in page order. Default generated filenames should be stable and human-readable, using 1-based visible page numbers, for example:

```text
sample.page-0001.svg
sample.page-0002.svg
```

The API should preserve the existing file-output policy:

- Python may normalize `PathLike` inputs.
- `overwrite=False` protects existing files.
- Rust core owns conversion loops and output planning.
- Python must not loop over pages through repeated FFI calls.

## Runtime dependency policy

SVG output must not require `rsvg-convert`. `rsvg-convert` is a PDF backend runtime dependency for SVG-to-PDF conversion only.

This distinction is part of the value of the SVG API:

```text
HWP -> SVG broken: parser/layout/SVG rendering issue
SVG correct, PDF broken: SVG-to-PDF backend issue
```

## Boundary rules

The SVG API must not expose long-lived `rhwp` internals:

- no `rhwp` document AST objects
- no borrowed layout tree
- no renderer-owned buffers with upstream lifetimes
- no adapter-owned state exposed as public Python objects

The adapter may translate upstream SVG render output into compact hwp-ingest-owned page artifacts. Future semantic objects may add provenance metadata, but the initial SVG output API can remain artifact-oriented.

## Documentation policy

User-facing documentation should describe SVG as a rendered artifact output, not as a debug-only feature. It may mention that SVG is useful for inspecting the stage before PDF backend conversion.

Recommended wording:

```text
SVG output renders HWP pages as page-level SVG artifacts. It is useful when callers need a rendered intermediate artifact or need to inspect output before PDF backend conversion.
```

## Non-goals

This decision does not require:

- exposing `rhwp` internal AST, layout tree, or renderer state
- adding a `debug_` public API namespace
- changing the default PDF backend policy
- making SVG output a semantic layout API
- promising that SVG output has final PDF fidelity on every backend
- implementing batch conversion in Python

## Verification

Implementation should include focused checks for:

1. `to_svg_pages` returns at least one SVG page for a known HWP fixture.
2. The first returned page starts with an SVG document root after leading whitespace.
3. `page_index=0` returns exactly one SVG page and matches the first all-pages output.
4. `to_svg_files` writes stable `.svg` paths and honors `overwrite=False`.
5. SVG conversion works without `rsvg-convert` on `PATH`.
6. Existing PDF conversion tests still pass.

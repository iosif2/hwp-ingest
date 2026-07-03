# Semantic outputs and production pipeline direction

<!-- Source: docs/rhwp-python-binding-plan.md lines 122-157. Original content below is copied verbatim. -->

## Semantic outputs and agent integration

Semantic output은 PDF-only product 방향을 피하기 위한 core contract다. Output category는 다음과 같다.

- Rendered artifacts: PDF, SVG/page images when supported.
- Textual artifacts: text/Markdown-like extraction when upstream or adapters support it.
- Structural artifacts: page metadata, layout boxes, table/shape regions, headings/sections, source file metadata.
- Agent-facing objects: converted pages, chunks, bounding boxes, file IDs, source hashes, derived artifact keys를 연결하는 stable Python/Rust data objects.

Upstream-supported output은 `hwp-ingest-rhwp-adapter`에서 upstream shape를 먼저 흡수하고, `hwp-ingest` engine이 제품 semantic object로 정리한 다음 PyO3 wrapper와 Python facade/stub에 반영한다. Python은 Rust engine이 만들 수 없는 semantic meaning을 invent하지 않는다. Exact upstream surface가 확인되지 않은 항목은 unverified — confirm against rhwp public API before implementation 상태로 둔다.

Semantic chunk metadata는 AI agent가 provenance를 복원할 수 있을 만큼 충분해야 한다. 최소 대상은 original file identity, page index, byte/hash identity where available, output artifact path/key, extraction version, conversion version이다. Chunk, bounding box, artifact key는 retry와 converter version upgrade 후에도 비교 가능해야 한다.

## Production pipeline direction

Recommended operating model은 다음과 같다.

```text
collector/API pull
  → raw object storage
  → file-level conversion jobs
  → Rust conversion workers
  → derived artifacts and metadata
  → AI microbatch consumes ready semantic outputs
```

Conversion은 event/job driven file-level 작업으로 운영한다. AI analysis는 이미 변환된 artifacts와 semantic metadata를 microbatch로 소비한다. AI batch가 파일마다 Python `to_pdf_file()` 또는 `to_pdf_bytes()`를 직접 호출하는 구조를 문서화하지 않는다.

Status vocabulary는 schema가 아니라 운영 문서 guidance다.

- File statuses: `RAW_STORED`, `CONVERTING`, `CONVERTED`, `CONVERSION_FAILED`, `UNSUPPORTED`, `TIMEOUT`
- AI statuses: `AI_PENDING`, `AI_RUNNING`, `AI_COMPLETED`, `AI_PARTIAL`, `AI_FAILED`

Storage는 raw artifact와 derived artifact를 분리한다. Raw storage는 source/file identity, original hash, ingest timestamp를 유지한다. Derived storage는 converter version, extraction version, output type, page index 또는 chunk id, artifact path/key를 versioned metadata로 보관한다. 실패한 job과 개선된 converter version은 raw artifact에서 재시도 가능해야 한다.

Queue는 deployment guidance다. Kafka/Redpanda 같은 stream platform을 먼저 도입하지 않는다. 초기에는 job table과 `FOR UPDATE SKIP LOCKED` 또는 equivalent locking으로 file-level work를 분배하는 단순 구조를 선호한다. 이 guidance는 crate dependency decision이 아니다.

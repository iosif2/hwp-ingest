# Vendor and upstream sync policy

<!-- Source: docs/rhwp-python-binding-plan.md lines 39-73. Original content below is copied verbatim. -->

## Vendor and upstream sync policy

선택된 시작 전략은 `surgical vendor fork`다. 다음 implementation task에서 Git dependency를 필요한 MIT-licensed `rhwp` subset으로 대체한다.

Vendoring이 구현될 때 필요한 파일은 다음과 같다.

```text
vendor/rhwp-subset/README.vendor.md
vendor/rhwp-subset/LICENSE.rhwp
vendor/rhwp-subset/THIRD_PARTY_LICENSES.rhwp.md
NOTICE
THIRD_PARTY_LICENSES.md
licenses/rhwp-MIT.txt
```

`vendor/rhwp-subset/README.vendor.md`는 최소한 다음 metadata를 기록한다.

- Origin URL
- Upstream license: `MIT`
- Copyright: `Copyright (c) 2025-2026 Edward Kim`
- Imported tag/commit
- Imported date
- Imported scope
- Local changes

이 metadata는 local checkout의 `LICENSE:1-13` 및 `THIRD_PARTY_LICENSES.md:1-13`에 근거한다. `LICENSE.rhwp`, `THIRD_PARTY_LICENSES.rhwp.md`, root `NOTICE`, root `THIRD_PARTY_LICENSES.md`, `licenses/rhwp-MIT.txt`는 upstream license와 third-party notice를 hwp-ingest 배포물에서 추적 가능하게 만든다.

Git/import rule은 다음과 같다.

1. 첫 원본 import commit은 선택한 upstream subset을 수정 없이 보존한다.
2. 후속 commit에서 prune/adapt 작업을 수행한다.
3. hwp-ingest adapter, batch engine, semantic object code는 upstream-derived tree 밖에 둔다.
4. Upstream sync는 recorded vendor base tag/commit부터 diff한다.
5. Parser/layout/render changes를 port하고 fixture regression tests를 다시 실행한다.
6. Imported scope, local changes, license metadata를 함께 갱신한다.

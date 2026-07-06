from __future__ import annotations

import os
from pathlib import Path
import tomllib


metadata = tomllib.loads(Path("pyproject.toml").read_text())["project"]
version = metadata["version"]
expected_tag = f"v{version}"
actual_tag = os.environ.get("GITHUB_REF_NAME", "")

print(f"pyproject version: {version}")
print(f"expected tag: {expected_tag}")
print(f"actual tag: {actual_tag}")

if actual_tag != expected_tag:
    raise SystemExit(f"release tag {actual_tag!r} does not match pyproject version {expected_tag!r}")

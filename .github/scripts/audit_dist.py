from __future__ import annotations

import argparse
from pathlib import Path
import tarfile
import tomllib
import zipfile


REQUIRED_WHEEL_PAYLOAD = [
    "hwp_ingest/__init__.py",
    "hwp_ingest/__init__.pyi",
    "hwp_ingest/_facade.py",
    "hwp_ingest/_native.pyi",
    "hwp_ingest/py.typed",
]

REQUIRED_LICENSE_NAMES = [
    "LICENSE",
    "NOTICE",
    "THIRD_PARTY_LICENSES.md",
    "licenses/rhwp-MIT.txt",
    "vendor/rhwp-subset/NOTICE",
]

EXPECTED_WHEEL_TAG_FRAGMENTS = [
    "manylinux_2_28_x86_64",
    "manylinux_2_28_aarch64",
    "macosx",
    "x86_64",
    "arm64",
    "win_amd64",
]


def assert_contains(names: set[str], item: str, context: str) -> None:
    if item not in names:
        raise SystemExit(f"missing {item} in {context}")


def audit_wheel(path: Path, version: str) -> None:
    print(f"auditing wheel: {path.name}")
    with zipfile.ZipFile(path) as archive:
        names = set(archive.namelist())
        for item in REQUIRED_WHEEL_PAYLOAD:
            assert_contains(names, item, path.name)
        if not any(name.startswith("hwp_ingest/_native") and name.endswith((".so", ".pyd", ".dylib")) for name in names):
            raise SystemExit(f"missing native extension in {path.name}")
        for license_name in REQUIRED_LICENSE_NAMES:
            if not any(name.endswith(f"dist-info/licenses/{license_name}") for name in names):
                raise SystemExit(f"missing license payload {license_name} in {path.name}")
        metadata_name = next(name for name in names if name.endswith(".dist-info/METADATA"))
        metadata = archive.read(metadata_name).decode()
        if f"Version: {version}" not in metadata:
            raise SystemExit(f"wheel {path.name} metadata does not contain Version: {version}")
        if "Classifier: Development Status :: 3 - Alpha" not in metadata:
            raise SystemExit(f"wheel {path.name} is missing alpha classifier")


def audit_sdist(path: Path, version: str) -> None:
    print(f"auditing sdist: {path.name}")
    with tarfile.open(path) as archive:
        names = set(archive.getnames())
    root = f"hwp_ingest-{version}"
    required = [
        f"{root}/README.md",
        f"{root}/LICENSE",
        f"{root}/NOTICE",
        f"{root}/THIRD_PARTY_LICENSES.md",
        f"{root}/licenses/rhwp-MIT.txt",
        f"{root}/vendor/rhwp-subset/README.vendor.md",
        f"{root}/vendor/rhwp-subset/NOTICE",
        f"{root}/vendor/rhwp-subset/src/lib.rs",
        f"{root}/python/hwp_ingest/__init__.pyi",
        f"{root}/python/hwp_ingest/_native.pyi",
        f"{root}/python/hwp_ingest/py.typed",
    ]
    for item in required:
        assert_contains(names, item, path.name)


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit release distributions before publishing.")
    parser.add_argument("--dist", default="dist")
    args = parser.parse_args()

    version = tomllib.loads(Path("pyproject.toml").read_text())["project"]["version"]
    dist = Path(args.dist)
    wheels = sorted(dist.glob("*.whl"))
    sdists = sorted(dist.glob("*.tar.gz"))

    if len(sdists) != 1:
        raise SystemExit(f"expected exactly one sdist, found {[path.name for path in sdists]}")
    if len(wheels) < 5:
        raise SystemExit(f"expected at least five wheels, found {[path.name for path in wheels]}")

    wheel_names = "\n".join(path.name for path in wheels)
    for fragment in EXPECTED_WHEEL_TAG_FRAGMENTS:
        if fragment not in wheel_names:
            raise SystemExit(f"missing wheel tag fragment {fragment!r}; wheels:\n{wheel_names}")

    audit_sdist(sdists[0], version)
    for wheel in wheels:
        audit_wheel(wheel, version)


if __name__ == "__main__":
    main()

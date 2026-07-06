from __future__ import annotations

import argparse
import glob
import os
from pathlib import Path
import shutil
import subprocess
import sys


def run(command: list[str]) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, check=True)


def venv_python(venv: Path) -> Path:
    if os.name == "nt":
        return venv / "Scripts" / "python.exe"
    return venv / "bin" / "python"


def main() -> None:
    parser = argparse.ArgumentParser(description="Install and smoke-test a built hwp-ingest wheel.")
    parser.add_argument("--dist", default="dist", help="Directory containing exactly one wheel")
    parser.add_argument(
        "--skip-conversion",
        action="store_true",
        help="Only import the wheel; skip rsvg-backed PDF conversion",
    )
    args = parser.parse_args()

    wheels = sorted(glob.glob(str(Path(args.dist) / "*.whl")))
    if len(wheels) != 1:
        raise SystemExit(f"expected exactly one wheel in {args.dist}, found {wheels}")

    venv = Path(".venv-wheel-smoke")
    shutil.rmtree(venv, ignore_errors=True)
    run([sys.executable, "-m", "venv", str(venv)])
    python = venv_python(venv)

    run(["uv", "pip", "install", "--python", str(python), wheels[0]])
    run([str(python), "-c", "import hwp_ingest; print(hwp_ingest.__all__)"])

    if args.skip_conversion:
        return

    fixture = Path("tests/fixtures/atop-equation-01.hwp")
    output = Path("wheel-smoke.pdf")
    code = (
        "from pathlib import Path; "
        "import hwp_ingest; "
        f"out = Path({str(output)!r}); "
        f"print(hwp_ingest.to_pdf_file(Path({str(fixture)!r}), out, overwrite=True)); "
        "data = out.read_bytes(); "
        "print(out.exists(), len(data), data[:5]); "
        "assert data.startswith(b'%PDF-')"
    )
    run([str(python), "-c", code])


if __name__ == "__main__":
    main()

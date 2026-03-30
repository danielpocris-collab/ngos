from __future__ import annotations

import argparse
import hashlib
import shutil
import sys
from pathlib import Path

try:
    from pyfatfs.PyFat import PyFat
    from pyfatfs.PyFatFS import PyFatFS
except ImportError as exc:  # pragma: no cover - build helper path
    raise SystemExit(
        "pyfatfs is required. Install it with: python -m pip install pyfatfs"
    ) from exc


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a FAT ESP image from a staged directory.")
    parser.add_argument("--source", required=True, type=Path, help="Staged directory root.")
    parser.add_argument("--output", required=True, type=Path, help="Output FAT image path.")
    parser.add_argument(
        "--size-mib",
        type=int,
        default=128,
        help="ESP image size in MiB. Defaults to 128.",
    )
    parser.add_argument(
        "--skip-verify",
        action="store_true",
        help="Skip post-build verification that image contents match the staged tree.",
    )
    return parser.parse_args()


def iter_source_entries(source: Path) -> list[tuple[Path, str]]:
    entries: list[tuple[Path, str]] = []
    for item in sorted(source.rglob("*")):
        relative = item.relative_to(source).as_posix()
        if relative:
            entries.append((item, relative))
    return entries


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def populate_image(source: Path, output: Path, size_mib: int) -> None:
    source = source.resolve()
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)

    if output.exists():
        output.unlink()

    size_bytes = size_mib * 1024 * 1024
    with open(output, "wb") as handle:
        handle.truncate(size_bytes)

    pyfat = PyFat()
    pyfat.mkfs(
        str(output),
        fat_type=PyFat.FAT_TYPE_FAT32,
        size=size_bytes,
        label="NGOSBOOT",
    )
    pyfat.close()

    fat_fs = PyFatFS(str(output), read_only=False)
    try:
        for item, relative in iter_source_entries(source):
            if item.is_dir():
                fat_fs.makedirs(relative, recreate=True)
                continue

            parent = Path(relative).parent.as_posix()
            if parent != ".":
                fat_fs.makedirs(parent, recreate=True)

            with item.open("rb") as src_handle, fat_fs.openbin(relative, "w") as dst_handle:
                shutil.copyfileobj(src_handle, dst_handle)
    finally:
        fat_fs.close()


def verify_image(source: Path, output: Path) -> None:
    source = source.resolve()
    output = output.resolve()
    fat_fs = PyFatFS(str(output), read_only=True)
    try:
        for item, relative in iter_source_entries(source):
            if item.is_dir():
                if not fat_fs.isdir(relative):
                    raise RuntimeError(f"Missing directory in ESP image: {relative}")
                continue

            if not fat_fs.isfile(relative):
                raise RuntimeError(f"Missing file in ESP image: {relative}")

            expected = item.read_bytes()
            with fat_fs.openbin(relative, "r") as handle:
                actual = handle.read()

            if actual != expected:
                raise RuntimeError(
                    "ESP image content mismatch for "
                    f"{relative}: source={sha256_bytes(expected)} image={sha256_bytes(actual)}"
                )
    finally:
        fat_fs.close()


def main() -> int:
    args = parse_args()
    populate_image(args.source, args.output, args.size_mib)
    if not args.skip_verify:
        verify_image(args.source, args.output)
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Builds a simple Rustcore boot filesystem image.

The format is intentionally tiny for now:
    magic   : 4 bytes  ("RCFS")
    version : u16      (currently 1)
    count   : u16      (number of entries)
    entries : repeated count times
        name_len : u16
        name     : UTF-8 bytes
        size     : u32
        data     : raw bytes

The bootloader copies the payload verbatim into memory; future revisions can
extend this to include compression or capability metadata.
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

MAGIC = b"RCFS"
VERSION = 1


def collect_entries(root: Path) -> list[tuple[str, bytes]]:
    entries: list[tuple[str, bytes]] = []
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        data = path.read_bytes()
        entries.append((rel, data))
    return entries


def write_image(entries: list[tuple[str, bytes]], output: Path) -> None:
    with output.open("wb") as fh:
        fh.write(MAGIC)
        fh.write(struct.pack("<H", VERSION))
        fh.write(struct.pack("<H", len(entries)))
        for name, data in entries:
            encoded = name.encode("utf-8")
            fh.write(struct.pack("<H", len(encoded)))
            fh.write(encoded)
            fh.write(struct.pack("<I", len(data)))
            fh.write(data)


def main() -> None:
    parser = argparse.ArgumentParser(description="Build Rustcore bootfs image")
    parser.add_argument("root", type=Path, help="Directory to package")
    parser.add_argument("output", type=Path, help="Output bootfs image path")
    args = parser.parse_args()

    if not args.root.is_dir():
        raise SystemExit(f"bootfs root '{args.root}' does not exist")

    entries = collect_entries(args.root)
    if not entries:
        raise SystemExit(f"bootfs root '{args.root}' did not contain any files")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_image(entries, args.output)


if __name__ == "__main__":
    main()

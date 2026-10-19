#!/usr/bin/env python3
"""Fail if a macOS wheel's extension has a misaligned LINKEDIT string pool.

Usage: check_macho_strtab.py <wheel> [<wheel> ...]

macOS 27's dyld refuses to load a 64-bit Mach-O image linked against the
macOS 27 SDK or later when its string pool offset (LC_SYMTAB stroff) is not
8-byte aligned. Older dyld versions load the same file without complaint, so
an import smoke test on an older runner cannot catch it. Wheels without Mach-O members
are skipped. Runs on the stock Python of every CI platform (no third-party
imports).
"""

import struct
import sys
import zipfile

MH_MAGIC_64 = 0xFEEDFACF
FAT_MAGIC = 0xCAFEBABE
FAT_MAGIC_64 = 0xCAFEBABF
LC_SYMTAB = 0x2


def thin_images(data: bytes):
    """Yield (offset, bytes) for each 64-bit Mach-O image in a thin or fat file."""
    if len(data) < 8:
        return
    magic = struct.unpack_from(">I", data)[0]
    if magic in (FAT_MAGIC, FAT_MAGIC_64):
        nfat = struct.unpack_from(">I", data, 4)[0]
        entry, fmt = (20, ">iiIII") if magic == FAT_MAGIC else (32, ">iiQQII")
        for i in range(nfat):
            _cpu, _sub, offset, size = struct.unpack_from(fmt, data, 8 + i * entry)[:4]
            yield from thin_images(data[offset:offset + size])
    elif struct.unpack_from("<I", data)[0] == MH_MAGIC_64:
        yield data


def string_pool_offsets(image: bytes):
    """Yield the stroff of every LC_SYMTAB in a 64-bit Mach-O image."""
    ncmds = struct.unpack_from("<I", image, 16)[0]
    pos = 32
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", image, pos)
        if cmd == LC_SYMTAB:
            yield struct.unpack_from("<I", image, pos + 16)[0]
        pos += cmdsize


def check_wheel(path: str) -> int:
    failures = 0
    with zipfile.ZipFile(path) as wheel:
        for member in wheel.namelist():
            if not member.endswith((".so", ".dylib")):
                continue
            for image in thin_images(wheel.read(member)):
                for stroff in string_pool_offsets(image):
                    if stroff % 8:
                        print(f"::error::{path}: {member} string pool offset "
                              f"0x{stroff:08X} is not 8-byte aligned")
                        failures += 1
                    else:
                        print(f"ok {path}: {member} string pool offset 0x{stroff:08X}")
    return failures


def main() -> None:
    failures = sum(check_wheel(path) for path in sys.argv[1:])
    if failures:
        raise SystemExit(f"{failures} misaligned string pool(s)")


if __name__ == "__main__":
    main()

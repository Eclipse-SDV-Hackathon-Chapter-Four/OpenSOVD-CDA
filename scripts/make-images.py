#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
"""Build the AZ3166 factory image and the app update package.

factory image (1 MiB, flashed over USB): bootloader | app in slot A | erased
slot B and state sector (so the update state starts fresh).

update package (flashed through the CDA, see crates/az3166-ecu/src/update.rs):

    offset  size  field (little endian)
         0     8  magic "AZ3166UP"
         8    16  version, NUL padded
        24     4  entry count
        28  16*4  entries: link base, offset in package, size, CRC-32
        92     4  CRC-32 of bytes 0..92
        96     -  images
"""

import argparse
import struct
import zlib
from pathlib import Path

FLASH_SIZE = 0x100000
BOOT_SIZE = 0x60000
SLOT_A = 0x08060000
SLOT_B = 0x080A0000
SLOT_SIZE = 0x40000
FLASH_BASE = 0x08000000

MAGIC = b"AZ3166UP"
HEADER_SIZE = 96
MAX_ENTRIES = 4


def package(version: str, images: list[tuple[int, bytes]]) -> bytes:
    header = bytearray(HEADER_SIZE)
    header[0:8] = MAGIC
    header[8:24] = version.encode()[:16].ljust(16, b"\0")
    struct.pack_into("<I", header, 24, len(images))
    offset = HEADER_SIZE
    for i, (base, image) in enumerate(images):
        struct.pack_into("<IIII", header, 28 + 16 * i, base, offset, len(image), zlib.crc32(image))
        offset += len(image)
    struct.pack_into("<I", header, 92, zlib.crc32(bytes(header[:92])))
    return bytes(header) + b"".join(image for _, image in images)


def fit(data: bytes, size: int, what: str) -> bytes:
    if len(data) > size:
        raise SystemExit(f"{what} is {len(data)} bytes, more than {size}")
    return data + b"\xff" * (size - len(data))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--version", required=True)
    parser.add_argument("--boot", type=Path, required=True)
    parser.add_argument("--app-a", type=Path, required=True)
    parser.add_argument("--app-b", type=Path, required=True)
    parser.add_argument("--factory", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    args = parser.parse_args()

    boot = args.boot.read_bytes()
    app_a = args.app_a.read_bytes()
    app_b = args.app_b.read_bytes()

    factory = fit(boot, BOOT_SIZE, "bootloader") + fit(app_a, SLOT_SIZE, "app (slot A)")
    factory = fit(factory, FLASH_SIZE, "factory image")
    args.factory.write_bytes(factory)

    fit(app_b, SLOT_SIZE, "app (slot B)")
    args.package.parent.mkdir(parents=True, exist_ok=True)
    args.package.write_bytes(package(args.version, [(SLOT_A, app_a), (SLOT_B, app_b)]))

    print(f"factory image {args.factory} (bootloader {len(boot)} B, app {len(app_a)} B)")
    print(f"update package {args.package} ({args.package.stat().st_size} B, version {args.version})")


if __name__ == "__main__":
    main()

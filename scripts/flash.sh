#!/usr/bin/env bash
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
#
# Flash the factory image (bootloader + app in slot A, update state reset).
#
#   scripts/flash.sh                 # OpenOCD over the on-board ST-LINK (SWD)
#   scripts/flash.sh --drive [path]  # copy to the AZ3166 USB drive instead
#
# OpenOCD (brew install open-ocd / apt install openocd) is preferred: the
# board's USB drive tends to disappear after a copy until it is replugged.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/build/az3166-factory.bin"

[[ -f "${BIN}" ]] || { echo "No ${BIN}: run scripts/build.sh" >&2; exit 1; }

flash_drive() {
    local drive="${1:-}"
    if [[ -z "${drive}" ]]; then
        for candidate in /Volumes/AZ3166 "/media/${USER}/AZ3166" "/run/media/${USER}/AZ3166"; do
            [[ -d "${candidate}" ]] && drive="${candidate}" && break
        done
    fi
    [[ -n "${drive}" && -d "${drive}" ]] || {
        echo "AZ3166 drive not found: replug the board or pass the mount point" >&2
        exit 1
    }
    cp "${BIN}" "${drive}/"
    sync
    echo "Copied to ${drive}; the board reprograms and restarts."
}

if [[ "${1:-}" == "--drive" ]]; then
    flash_drive "${2:-}"
elif command -v openocd >/dev/null; then
    openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
        -c "program ${BIN} 0x08000000 verify reset exit"
else
    echo "openocd not found, using the USB drive" >&2
    flash_drive ""
fi
echo "Console: 115200 8N1 on the board's USB serial port."

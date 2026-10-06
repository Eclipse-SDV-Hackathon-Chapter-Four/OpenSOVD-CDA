#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Flash build/flxc1000.bin by copying it to the AZ3166 USB drive
# (the on-board ST-LINK programs the MCU and resets it).
#
#   scripts/flash.sh [mount-point]   # default: /Volumes/AZ3166 or /media/$USER/AZ3166

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/build/flxc1000.bin"

[[ -f "${BIN}" ]] || { echo "No ${BIN}: run scripts/build.sh" >&2; exit 1; }

DRIVE="${1:-}"
if [[ -z "${DRIVE}" ]]; then
    for candidate in /Volumes/AZ3166 "/media/${USER}/AZ3166" "/run/media/${USER}/AZ3166"; do
        [[ -d "${candidate}" ]] && DRIVE="${candidate}" && break
    done
fi
[[ -n "${DRIVE}" && -d "${DRIVE}" ]] || { echo "AZ3166 drive not found; pass the mount point" >&2; exit 1; }

cp "${BIN}" "${DRIVE}/"
sync
echo "Copied to ${DRIVE}; the board reprograms and restarts (LED blinks)."
echo "Console: 115200 8N1 on the board's USB serial port."

#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Download the Arm GNU Toolchain (arm-none-eabi, with newlib) into
# ./.toolchain so the firmware builds without a system-wide install.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="14.3.rel1"

case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)
        HOST="darwin-arm64"
        SHA256="30f4d08b219190a37cded6aa796f4549504902c53cfc3c7e044a8490b6eba1f7"
        ;;
    Darwin-x86_64)
        HOST="darwin-x86_64"
        SHA256=""
        ;;
    Linux-x86_64)
        HOST="x86_64"
        SHA256=""
        ;;
    Linux-aarch64)
        HOST="aarch64"
        SHA256=""
        ;;
    *)
        echo "Unsupported host: $(uname -s)-$(uname -m)" >&2
        exit 1
        ;;
esac

NAME="arm-gnu-toolchain-${VERSION}-${HOST}-arm-none-eabi"
URL="https://developer.arm.com/-/media/Files/downloads/gnu/${VERSION}/binrel/${NAME}.tar.xz"
DEST="${ROOT}/.toolchain"

if [[ -x "${DEST}/${NAME}/bin/arm-none-eabi-gcc" ]]; then
    echo "Toolchain already present: ${DEST}/${NAME}"
    exit 0
fi

mkdir -p "${DEST}"
ARCHIVE="${DEST}/${NAME}.tar.xz"

echo "Downloading ${URL}"
curl -fL --retry 3 -o "${ARCHIVE}" "${URL}"

if [[ -z "${SHA256}" ]]; then
    SHA256="$(curl -fsL "${URL}.sha256asc" | awk '{print $1}')"
fi
echo "${SHA256}  ${ARCHIVE}" | shasum -a 256 -c -

tar -xJf "${ARCHIVE}" -C "${DEST}"
rm -f "${ARCHIVE}"

echo "Installed: ${DEST}/${NAME}/bin"

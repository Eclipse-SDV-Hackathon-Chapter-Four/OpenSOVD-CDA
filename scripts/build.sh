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
# Build the AZ3166 firmware: bootloader, app (slots A/B), the factory image
# build/az3166-factory.bin and the update package build/update/az3166-app-<version>.bin
#
#   WIFI_SSID=MyNet WIFI_PASSWORD=secret scripts/build.sh
#
# The credentials are compiled in. They are kept in the CMake cache of
# ./build, so later builds without the variables reuse them.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

[[ -d third_party/threadx ]] || scripts/fetch-deps.sh
ls .toolchain/arm-gnu-toolchain-*/bin/arm-none-eabi-gcc >/dev/null 2>&1 \
    || command -v arm-none-eabi-gcc >/dev/null \
    || scripts/setup-toolchain.sh

ARGS=(-S platform -B build -G Ninja)
[[ -n "${WIFI_SSID:-}" ]] && ARGS+=(-DWIFI_SSID="${WIFI_SSID}")
[[ -n "${WIFI_PASSWORD+x}" ]] && ARGS+=(-DWIFI_PASSWORD="${WIFI_PASSWORD}")

cmake "${ARGS[@]}"
cmake --build build

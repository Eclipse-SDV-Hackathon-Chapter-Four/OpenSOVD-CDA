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
# Fetch the C dependencies into ./third_party (not committed):
#
#   threadx, netxduo  - pinned to the commits the prebuilt WICED Wi-Fi library
#                       was built against (ABI must match)
#   getting-started   - archived Azure RTOS AZ3166 board support: STM32CubeF4,
#                       startup code, linker script and the WICED library
#
# The WICED library and the BCM43362 firmware are Cypress property and may only
# be used with Cypress chips (the AZ3166's EMW3166 module). They are therefore
# downloaded here instead of being redistributed in this repository.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TP="${ROOT}/third_party"

THREADX_URL="https://github.com/eclipse-threadx/threadx.git"
THREADX_REV="37f6d0b39c970b297c263cbb8388b9060b92e5c9"

NETXDUO_URL="https://github.com/eclipse-threadx/netxduo.git"
NETXDUO_REV="2973652d801e92615caa285ad9ee13db19a26cb9"

GSG_URL="https://github.com/azure-rtos/getting-started.git"
GSG_REV="master" # archived, read-only

fetch() {
    local name="$1" url="$2" rev="$3"
    local dir="${TP}/${name}"

    if [[ -d "${dir}/.git" ]]; then
        echo "${name}: present"
        return
    fi

    echo "${name}: fetching ${rev}"
    git init -q "${dir}"
    git -C "${dir}" remote add origin "${url}"
    git -C "${dir}" fetch -q --depth 1 origin "${rev}"
    git -C "${dir}" checkout -q FETCH_HEAD
}

mkdir -p "${TP}"
fetch threadx "${THREADX_URL}" "${THREADX_REV}"
fetch netxduo "${NETXDUO_URL}" "${NETXDUO_REV}"
fetch getting-started "${GSG_URL}" "${GSG_REV}"

echo "Dependencies ready in ${TP}"

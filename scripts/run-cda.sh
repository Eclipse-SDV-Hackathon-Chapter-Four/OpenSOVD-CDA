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
# Run the Eclipse OpenSOVD Classic Diagnostic Adapter against the AZ3166 ECU.
#
#   scripts/run-cda.sh <ecu-ip> [extra CDA args...]
#
# Uses odx/AZ3166.mdd (or the *.mdd in $DATABASES_DIR), the tester interface
# that routes to <ecu-ip> (board or host_sim; $TESTER_IFACE overrides it)
# and build/update as flash files directory ($FLASH_DIR).
# SOVD API: http://localhost:20002 (swagger-ui).
#
# CDA checkout: $CDA_DIR (default ~/dev/classic-diagnostic-adapter). Uses
# its target/release or target/debug binary, building a debug one if absent.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CDA_DIR="${CDA_DIR:-${HOME}/dev/classic-diagnostic-adapter}"

ECU_IP="${1:?usage: run-cda.sh <ecu-ip> [extra CDA args...]}"
shift

# App update packages (scripts/az3166-flash) are served from here.
FLASH_DIR="${FLASH_DIR:-${ROOT}/build/update}"
# Diagnostic databases (*.mdd); defaults to the AZ3166 MDD in odx/.
DATABASES_DIR="${DATABASES_DIR:-${ROOT}/odx}"
mkdir -p "${FLASH_DIR}"

if [[ -x "${CDA_DIR}/target/release/opensovd-cda" ]]; then
    CDA="${CDA_DIR}/target/release/opensovd-cda"
elif [[ -x "${CDA_DIR}/target/debug/opensovd-cda" ]]; then
    CDA="${CDA_DIR}/target/debug/opensovd-cda"
else
    (cd "${CDA_DIR}" && cargo build --bin opensovd-cda)
    CDA="${CDA_DIR}/target/debug/opensovd-cda"
fi

# Tester address and subnet: the local interface that routes to the ECU.
if [[ "${ECU_IP}" == 127.* ]]; then
    TESTER_IP="127.0.0.1"
    TESTER_MASK="255.0.0.0"
else
    # TESTER_IFACE (e.g. en0) overrides the interface the route lookup finds.
    IFACE="${TESTER_IFACE:-$(route -n get "${ECU_IP}" 2>/dev/null | awk '/interface:/ {print $2}')}"
    [[ -n "${IFACE}" ]] || { echo "No route to ${ECU_IP}" >&2; exit 1; }
    read -r TESTER_IP HEXMASK < <(ifconfig "${IFACE}" | awk '/inet / {print $2, $4; exit}')
    TESTER_MASK="$(printf '%d.%d.%d.%d' $((HEXMASK >> 24 & 255)) $((HEXMASK >> 16 & 255)) \
        $((HEXMASK >> 8 & 255)) $((HEXMASK & 255)))"
    echo "Tester interface ${IFACE}: ${TESTER_IP}/${TESTER_MASK}"
fi

exec "${CDA}" \
    --databases-dir "${DATABASES_DIR}" \
    --flash-files-path "${FLASH_DIR}" \
    --tester-address "${TESTER_IP}" \
    --tester-subnet "${TESTER_MASK}" \
    --protocol-name UDS_Ethernet_DoIP \
    --listen-address 127.0.0.1 \
    --listen-port 20002 \
    "$@"

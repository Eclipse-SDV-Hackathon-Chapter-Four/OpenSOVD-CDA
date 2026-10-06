#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Run the Eclipse OpenSOVD Classic Diagnostic Adapter against the FLXC1000.
#
#   scripts/run-cda.sh <ecu-ip> [extra CDA args...]
#
# Uses odx/AZ3166.mdd and the tester interface that routes to <ecu-ip>
# (board or host_sim). SOVD API: http://localhost:20002 (swagger-ui).
#
# CDA checkout: $CDA_DIR (default ~/dev/classic-diagnostic-adapter). Uses
# its target/release or target/debug binary, building a debug one if absent.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CDA_DIR="${CDA_DIR:-${HOME}/dev/classic-diagnostic-adapter}"

ECU_IP="${1:?usage: run-cda.sh <ecu-ip> [extra CDA args...]}"
shift

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
    IFACE="$(route -n get "${ECU_IP}" 2>/dev/null | awk '/interface:/ {print $2}')"
    [[ -n "${IFACE}" ]] || { echo "No route to ${ECU_IP}" >&2; exit 1; }
    read -r TESTER_IP HEXMASK < <(ifconfig "${IFACE}" | awk '/inet / {print $2, $4; exit}')
    TESTER_MASK="$(printf '%d.%d.%d.%d' $((HEXMASK >> 24 & 255)) $((HEXMASK >> 16 & 255)) \
        $((HEXMASK >> 8 & 255)) $((HEXMASK & 255)))"
    echo "Tester interface ${IFACE}: ${TESTER_IP}/${TESTER_MASK}"
fi

exec "${CDA}" \
    --databases-dir "${ROOT}/odx" \
    --tester-address "${TESTER_IP}" \
    --tester-subnet "${TESTER_MASK}" \
    --protocol-name UDS_Ethernet_DoIP \
    --listen-address 127.0.0.1 \
    --listen-port 20002 \
    "$@"

#!/bin/sh -e
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Local build: creates odx/.venv (odxtools pinned in requirements.txt),
# generates FLXC1000.pdx, verifies it and, if an odx-converter jar is
# available, converts it to FLXC1000.mdd for the Classic Diagnostic Adapter.
#
#   ./generate.sh                                   # PDX only
#   ODX_CONVERTER_JAR=/path/to/converter-all.jar ./generate.sh   # PDX + MDD

SCRIPT_DIR=$(dirname "$(realpath "$0")")
cd "$SCRIPT_DIR"

if [ ! -x .venv/bin/python ]; then
    if command -v uv >/dev/null 2>&1; then
        uv venv --python 3.12 .venv
        uv pip install --python .venv/bin/python -r requirements.txt
    else
        python3 -m venv .venv
        .venv/bin/pip install -r requirements.txt
    fi
fi

.venv/bin/python generate.py FLXC1000.pdx
.venv/bin/python verify.py FLXC1000.pdx

if [ -n "$ODX_CONVERTER_JAR" ]; then
    # Newer odx-converter builds have subcommands; older ones take the PDX directly.
    if java -jar "$ODX_CONVERTER_JAR" --help 2>&1 | grep -q "convert"; then
        java -jar "$ODX_CONVERTER_JAR" convert FLXC1000.pdx
    else
        java -jar "$ODX_CONVERTER_JAR" FLXC1000.pdx
    fi
else
    echo "ODX_CONVERTER_JAR not set, skipping PDX -> MDD conversion"
fi

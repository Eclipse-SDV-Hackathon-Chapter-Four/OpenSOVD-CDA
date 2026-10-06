#!/bin/sh -e
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Same as the reference testcontainer/odx/generate_docker.sh: builds a pinned
# odxtools image and runs generate.py inside it. Produces AZ3166.pdx.

SCRIPT_DIR=$(dirname "$(realpath "$0")")
docker build -f "$SCRIPT_DIR/docker/Dockerfile" "$SCRIPT_DIR" -t flxc1000-az3166-odx-gen
docker run --rm -v "$SCRIPT_DIR:/data" -u "$(id -u):$(id -g)" -t flxc1000-az3166-odx-gen

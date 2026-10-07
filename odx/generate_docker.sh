#!/bin/sh -e
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
# Same as the reference testcontainer/odx/generate_docker.sh: builds a pinned
# odxtools image and runs generate.py inside it. Produces AZ3166.pdx.

SCRIPT_DIR=$(dirname "$(realpath "$0")")
docker build -f "$SCRIPT_DIR/docker/Dockerfile" "$SCRIPT_DIR" -t az3166-odx-gen
docker run --rm -v "$SCRIPT_DIR:/data" -u "$(id -u):$(id -g)" -t az3166-odx-gen

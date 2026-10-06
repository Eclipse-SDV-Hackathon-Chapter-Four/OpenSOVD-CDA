# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Mirrors the reference testcontainer/odx/docker/Dockerfile of the Eclipse
# OpenSOVD Classic Diagnostic Adapter.

FROM python:3.12-slim-trixie

COPY requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir -r /tmp/requirements.txt

WORKDIR /data

ENTRYPOINT ["/usr/local/bin/python", "generate.py"]

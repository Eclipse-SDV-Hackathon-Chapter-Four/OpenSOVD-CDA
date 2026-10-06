/*
 * SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Allocation-free ISO 13400-2 DoIP entity. No I/O: the firmware feeds socket
//! data in and sends the frames that come out.

#![cfg_attr(not(test), no_std)]

pub mod connection;
pub mod header;
pub mod message;
pub mod udp;

pub use connection::{Connection, SendError, Transport, UdsHandler};
pub use udp::DoipConfig;

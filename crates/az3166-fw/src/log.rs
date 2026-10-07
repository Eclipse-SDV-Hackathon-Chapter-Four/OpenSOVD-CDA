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

//! Line logging to the UART console (USB serial, 115200 8N1).

use core::fmt::Write;

pub const LINE_CAPACITY: usize = 160;

pub fn write_line(args: core::fmt::Arguments) {
    let mut line = heapless::String::<LINE_CAPACITY>::new();
    // Overlong lines are truncated.
    let _ = line.write_fmt(args);
    let _ = line.push_str("\r\n");
    unsafe { crate::sys::plat_log(line.as_ptr(), line.len() as u32) };
}

macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write_line(format_args!($($arg)*))
    };
}

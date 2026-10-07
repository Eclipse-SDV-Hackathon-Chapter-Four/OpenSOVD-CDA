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

//! Presence task: once per second reads the occupancy from the presence
//! sensor (network) and the ambient temperature, and runs the alarm detector.

use core::sync::atomic::Ordering;

use az3166_ecu::presence::{Detector, ALARM_TRIGGERED};
use az3166_ecu::Board;

use crate::board::Az3166;
use crate::{sys, SHARED};

const PERIOD_MS: u32 = 1000;
/// Failed polls in a row before the occupancy counts as not available; a
/// single failed HTTP exchange keeps the last answer.
const MISSES_TOLERATED: u32 = 3;

/// 7.2 KiB of temperature history: static, not on the thread stack.
static mut DETECTOR: Detector = Detector::new();

#[no_mangle]
pub extern "C" fn az3166_presence_task() -> ! {
    // SAFETY: only this thread uses the detector.
    let detector = unsafe { &mut *core::ptr::addr_of_mut!(DETECTOR) };
    let alarm = &SHARED.alarm;
    let mut last = (u8::MAX, u8::MAX);
    let mut last_answer = None;
    let mut misses = 0;
    loop {
        let started = sys::uptime_us();
        // SAFETY: only called from this thread.
        unsafe { sys::plat_net_maintain() };
        let presence = match unsafe { sys::plat_presence_read() } {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        };
        let presence = match presence {
            Some(answer) => {
                misses = 0;
                last_answer = Some(answer);
                Some(answer)
            }
            None => {
                misses += 1;
                if misses < MISSES_TOLERATED {
                    last_answer
                } else {
                    None
                }
            }
        };
        let temperature = Az3166.temperature_humidity().map(|(t, _)| tenths(t));
        detector.step(alarm, presence, temperature);

        let state = (
            alarm.presence.load(Ordering::Relaxed),
            alarm.alarm.load(Ordering::Relaxed),
        );
        if state != last {
            log!(
                "Presence {} alarm {} ({} / rise {} x 0.1 C)",
                state.0,
                if state.1 == ALARM_TRIGGERED {
                    "TRIGGERED"
                } else {
                    "armed"
                },
                alarm.temperature.load(Ordering::Relaxed),
                alarm.rise.load(Ordering::Relaxed)
            );
            last = state;
        }
        let elapsed = (sys::uptime_us().saturating_sub(started) / 1000) as u32;
        sys::sleep_ms(PERIOD_MS.saturating_sub(elapsed).max(50));
    }
}

/// °C to 0.1 °C, rounded.
fn tenths(celsius: f32) -> i16 {
    let raw = celsius * 10.0;
    (if raw < 0.0 { raw - 0.5 } else { raw + 0.5 }) as i16
}

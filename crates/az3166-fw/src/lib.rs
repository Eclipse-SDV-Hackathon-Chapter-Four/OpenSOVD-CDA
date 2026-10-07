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

//! AZ3166 ECU firmware for the MXCHIP AZ3166.
//!
//! Built as a static library and linked into the C platform (`platform/`),
//! which owns the hardware, ThreadX, NetX Duo and the Wi-Fi driver. The
//! platform creates the threads and calls the `az3166_*` entry points:
//!
//! - UDS worker: [`az3166_init`], then [`az3166_uds_execute`] per request
//!   and [`az3166_uds_tick`] every 100 ms. Only this thread touches the ECU.
//! - [`az3166_tcp_task`] per DoIP TCP slot, [`az3166_udp_task`] for
//!   vehicle identification, [`az3166_routine_task`] for the LED self-test,
//!   [`az3166_speech_task`] (App only) renders speech to the audio output,
//!   [`az3166_presence_task`] (App only) runs the occupancy / temperature alarm.

#![no_std]

mod board;
#[macro_use]
mod log;
mod net;
mod presence;
mod screens;
mod speech;
mod sys;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

use az3166_ecu::{app, BootState, Ecu, Shared, VIN};

use board::Az3166;

pub use net::{az3166_tcp_task, az3166_udp_task};
pub use presence::az3166_presence_task;
pub use speech::az3166_speech_task;

pub(crate) static SHARED: Shared = Shared::new();

/// The ECU (~11 KiB). Written once by `az3166_init` and afterwards only
/// used on the UDS worker thread.
struct EcuCell(UnsafeCell<MaybeUninit<Ecu<Az3166>>>);

// SAFETY: accessed from the UDS worker thread only (see module docs).
unsafe impl Sync for EcuCell {}

static ECU: EcuCell = EcuCell(UnsafeCell::new(MaybeUninit::uninit()));
static ECU_READY: AtomicBool = AtomicBool::new(false);
static IS_APP: AtomicBool = AtomicBool::new(false);

/// # Safety
/// Must only be called on the UDS worker thread after `az3166_init`.
unsafe fn ecu() -> &'static mut Ecu<Az3166> {
    (*ECU.0.get()).assume_init_mut()
}

/// Builds the ECU for the boot state chosen by the platform.
#[no_mangle]
pub extern "C" fn az3166_init(boot_state: u32) {
    let state = match boot_state {
        sys::BOOT_STATE_BOOT_REQUESTED => BootState::BootRequested,
        _ => BootState::AppValid,
    };
    // SAFETY: called once by the UDS worker thread before any other entry point.
    let ecu = unsafe { (*ECU.0.get()).write(Ecu::new(state, Az3166, &SHARED)) };
    IS_APP.store(ecu.is_app(), Ordering::Relaxed);
    ECU_READY.store(true, Ordering::Release);

    log!(
        "az3166 {} variant, address 0x{:04X}, VIN {}",
        if ecu.is_app() { "App" } else { "Boot" },
        az3166_ecu::ECU_ADDRESS,
        core::str::from_utf8(VIN).unwrap_or("?")
    );
}

/// Handles one UDS request. UDS worker thread only.
///
/// # Safety
/// `req` must point to `len` readable bytes and `resp` to `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn az3166_uds_execute(
    source: u16,
    req: *const u8,
    len: u32,
    resp: *mut u8,
    cap: u32,
) -> u32 {
    let request = core::slice::from_raw_parts(req, len as usize);
    let response = core::slice::from_raw_parts_mut(resp, cap as usize);
    ecu().handle(source, request, response, sys::uptime_us()) as u32
}

/// Advances ECU timers. UDS worker thread only.
#[no_mangle]
pub extern "C" fn az3166_uds_tick() {
    if ECU_READY.load(Ordering::Acquire) {
        // SAFETY: called on the UDS worker thread.
        unsafe { ecu() }.tick(sys::uptime_us());
    }
}

const ROUTINE_PERIOD_MS: u32 = 50;

/// Lowest-priority task: kicks the watchdog, runs the SelfTest LED cascade
/// when started and drives the display screens (buttons A/B).
#[no_mangle]
pub extern "C" fn az3166_routine_task() -> ! {
    let is_app = IS_APP.load(Ordering::Relaxed);
    sys::watchdog_kick();
    if is_app {
        app::boot_blink(&Az3166);
    }
    let mut ui = screens::Ui::new(is_app);
    loop {
        // Starves (and the watchdog resets) if a higher-priority thread hangs.
        sys::watchdog_kick();
        if SHARED.routine_start.swap(false, Ordering::SeqCst) {
            log!("SelfTest LED cascade");
            app::run_self_test(&SHARED, &Az3166);
        }
        ui.update(&SHARED, ROUTINE_PERIOD_MS);
        sys::sleep_ms(ROUTINE_PERIOD_MS);
    }
}

/// After a response went out: performs an ECUReset requested by it.
pub(crate) fn handle_pending_reset() {
    if SHARED.take_pending_reset() != az3166_ecu::RESET_NONE {
        log!("ECUReset");
        // Let the response leave the network stack first.
        sys::sleep_ms(50);
        sys::reset();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    sys::sleep_ms(100);
    sys::reset()
}

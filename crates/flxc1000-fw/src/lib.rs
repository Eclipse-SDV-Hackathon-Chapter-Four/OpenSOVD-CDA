/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! FLXC1000 firmware for the MXCHIP AZ3166.
//!
//! Built as a static library and linked into the C platform (`platform/`),
//! which owns the hardware, ThreadX, NetX Duo and the Wi-Fi driver. The
//! platform creates the threads and calls the `flxc1000_*` entry points:
//!
//! - UDS worker: [`flxc1000_init`], then [`flxc1000_uds_execute`] per request
//!   and [`flxc1000_uds_tick`] every 100 ms. Only this thread touches the ECU.
//! - [`flxc1000_tcp_task`] per DoIP TCP slot, [`flxc1000_udp_task`] for
//!   vehicle identification, [`flxc1000_routine_task`] for the LED self-test.

#![no_std]

mod board;
#[macro_use]
mod log;
mod net;
mod sys;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use flxc1000_ecu::{app, BootState, Ecu, Shared, VIN};

use board::Az3166;

pub use net::{flxc1000_tcp_task, flxc1000_udp_task};

static SHARED: Shared = Shared::new();

/// The ECU (~136 KiB). Written once by `flxc1000_init` and afterwards only
/// used on the UDS worker thread.
struct EcuCell(UnsafeCell<MaybeUninit<Ecu<Az3166>>>);

// SAFETY: accessed from the UDS worker thread only (see module docs).
unsafe impl Sync for EcuCell {}

static ECU: EcuCell = EcuCell(UnsafeCell::new(MaybeUninit::uninit()));
static ECU_READY: AtomicBool = AtomicBool::new(false);
static IS_APP: AtomicBool = AtomicBool::new(false);
static DISPLAYED_SESSION: AtomicU8 = AtomicU8::new(0);

/// # Safety
/// Must only be called on the UDS worker thread after `flxc1000_init`.
unsafe fn ecu() -> &'static mut Ecu<Az3166> {
    (*ECU.0.get()).assume_init_mut()
}

/// Builds the ECU for the boot state chosen by the platform.
#[no_mangle]
pub extern "C" fn flxc1000_init(boot_state: u32) {
    let state = match boot_state {
        sys::BOOT_STATE_BOOT_REQUESTED => BootState::BootRequested,
        _ => BootState::AppValid,
    };
    // SAFETY: called once by the UDS worker thread before any other entry point.
    let ecu = unsafe { (*ECU.0.get()).write(Ecu::new(state, Az3166, &SHARED)) };
    IS_APP.store(ecu.is_app(), Ordering::Relaxed);
    ECU_READY.store(true, Ordering::Release);

    let (title, text): (&[u8], &[u8]) = if ecu.is_app() {
        (b"AZ3166 App", b"AZ3166")
    } else {
        (b"AZ3166 Boot", b"BOOTLOADER")
    };
    sys::display_line(0, title);
    sys::display_line(3, text);
    log!(
        "flxc1000 {} variant, address 0x{:04X}, VIN {}",
        if ecu.is_app() { "App" } else { "Boot" },
        flxc1000_ecu::ECU_ADDRESS,
        core::str::from_utf8(VIN).unwrap_or("?")
    );
}

/// Handles one UDS request. UDS worker thread only.
///
/// # Safety
/// `req` must point to `len` readable bytes and `resp` to `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn flxc1000_uds_execute(
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
pub extern "C" fn flxc1000_uds_tick() {
    if ECU_READY.load(Ordering::Acquire) {
        // SAFETY: called on the UDS worker thread.
        unsafe { ecu() }.tick(sys::uptime_us());
    }
}

/// Runs the SelfTest LED cascade when started and keeps the session shown on
/// the display current.
#[no_mangle]
pub extern "C" fn flxc1000_routine_task() -> ! {
    if IS_APP.load(Ordering::Relaxed) {
        app::boot_blink(&Az3166);
    }
    loop {
        if SHARED.routine_start.swap(false, Ordering::SeqCst) {
            log!("SelfTest LED cascade");
            app::run_self_test(&SHARED, &Az3166);
        }
        update_session_display();
        sys::sleep_ms(50);
    }
}

fn update_session_display() {
    let session = SHARED.session_type.load(Ordering::Relaxed);
    if DISPLAYED_SESSION.swap(session, Ordering::Relaxed) == session {
        return;
    }
    let text: &[u8] = match session {
        0x01 => b"Session: Default",
        0x02 => b"Session: Program",
        0x03 => b"Session: Extended",
        _ => b"Session: ?",
    };
    sys::display_line(2, text);
}

/// After a response went out: performs an ECUReset requested by it.
pub(crate) fn handle_pending_reset() {
    if SHARED.take_pending_reset() != flxc1000_ecu::RESET_NONE {
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

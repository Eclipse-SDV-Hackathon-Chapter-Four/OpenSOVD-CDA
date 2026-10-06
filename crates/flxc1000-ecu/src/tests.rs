/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Host tests against `docs/diagnostics.md`.

use std::sync::{Arc, Mutex};
use std::vec::Vec;

use super::*;

#[derive(Default)]
struct BoardState {
    leds: [u8; board::LED_COUNT],
    led_history: Vec<(usize, u8)>,
    rgb: [u8; 3],
    text: [u8; 16],
    boot_state: Option<BootState>,
    broken: Vec<Sensor>,
    buttons: u8,
}

#[derive(Clone, Default)]
struct FakeBoard(Arc<Mutex<BoardState>>);

impl FakeBoard {
    fn state(&self) -> std::sync::MutexGuard<'_, BoardState> {
        self.0.lock().unwrap()
    }
}

impl Board for FakeBoard {
    fn set_led(&self, index: usize, duty: u8) {
        let mut s = self.state();
        s.leds[index] = duty;
        s.led_history.push((index, duty));
    }
    fn sleep_ms(&self, _ms: u32) {}
    fn uptime_ms(&self) -> u64 {
        42_500
    }
    fn random_u32(&self) -> u32 {
        0x1234_5678
    }
    fn serial_number(&self) -> [u8; 12] {
        [0xAB; 12]
    }
    fn ip_address(&self) -> [u8; 4] {
        [192, 168, 1, 50]
    }
    fn mac_address(&self) -> [u8; 6] {
        [0xC8, 0x93, 0x46, 0x01, 0x02, 0x03]
    }
    fn sensor_ok(&self, sensor: Sensor) -> bool {
        !self.state().broken.contains(&sensor)
    }
    fn temperature_humidity(&self) -> Option<(f32, f32)> {
        self.sensor_ok(Sensor::HumidityTemperature)
            .then_some((-12.34, 45.6))
    }
    fn pressure_hpa(&self) -> Option<f32> {
        self.sensor_ok(Sensor::Pressure).then_some(1013.25)
    }
    fn acceleration_mg(&self) -> Option<[f32; 3]> {
        self.sensor_ok(Sensor::Inertial)
            .then_some([1.0, -2.0, 1000.0])
    }
    fn angular_rate_mdps(&self) -> Option<[f32; 3]> {
        self.sensor_ok(Sensor::Inertial)
            .then_some([700.0, -1400.0, 0.0])
    }
    fn magnetic_field_mg(&self) -> Option<[f32; 3]> {
        self.sensor_ok(Sensor::Magnetometer)
            .then_some([100.0, 200.0, -300.0])
    }
    fn buttons(&self) -> u8 {
        self.state().buttons
    }
    fn set_rgb(&self, rgb: [u8; 3]) {
        self.state().rgb = rgb;
    }
    fn set_display_text(&self, text: &[u8; 16]) {
        self.state().text = *text;
    }
    fn write_boot_state(&self, state: BootState) -> Result<(), FlashError> {
        self.state().boot_state = Some(state);
        Ok(())
    }
}

struct Harness {
    ecu: Box<Ecu<FakeBoard>>,
    board: FakeBoard,
    shared: &'static Shared,
    now_us: u64,
}

impl Harness {
    fn new(state: BootState) -> Self {
        Self::with_board(state, FakeBoard::default())
    }

    fn with_board(state: BootState, board: FakeBoard) -> Self {
        let shared: &'static Shared = Box::leak(Box::new(Shared::new()));
        Self {
            ecu: Box::new(Ecu::new(state, board.clone(), shared)),
            board,
            shared,
            now_us: 1_000_000,
        }
    }

    fn req(&mut self, request: &[u8]) -> Vec<u8> {
        self.now_us += 10_000;
        let mut resp = vec![0u8; 4096];
        let n = self.ecu.handle(0x0E00, request, &mut resp, self.now_us);
        resp.truncate(n);
        resp
    }
}

#[test]
fn app_identification() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(
        h.req(&[0x22, 0xF1, 0x00]),
        [0x62, 0xF1, 0x00, 0x00, 0x01, 0x01]
    );
    assert_eq!(h.req(&[0x22, 0xF1, 0x86]), [0x62, 0xF1, 0x86, 0x01]);
    let mut vin = vec![0x62, 0xF1, 0x90];
    vin.extend_from_slice(VIN);
    assert_eq!(h.req(&[0x22, 0xF1, 0x90]), vin);
    assert_eq!(&h.req(&[0x22, 0xF1, 0x8C])[3..], &[0xAB; 12]);
    assert_eq!(&h.req(&[0x22, 0xF1, 0x95])[3..], b"0.1.0   ");
}

#[test]
fn app_sensor_scaling() {
    let mut h = Harness::new(BootState::AppValid);
    // -12.34 °C → -123 (0.1 °C)
    assert_eq!(&h.req(&[0x22, 0xF2, 0x01])[3..], &(-123i16).to_be_bytes());
    // 45.6 %RH → 456
    assert_eq!(&h.req(&[0x22, 0xF2, 0x02])[3..], &456u16.to_be_bytes());
    // 1013.25 hPa → 10133 (rounded)
    assert_eq!(&h.req(&[0x22, 0xF2, 0x03])[3..], &10133u16.to_be_bytes());
    assert_eq!(
        &h.req(&[0x22, 0xF2, 0x04])[3..],
        &[0x00, 0x01, 0xFF, 0xFE, 0x03, 0xE8]
    );
    // 700 mdps → 7 (0.1 dps)
    assert_eq!(
        &h.req(&[0x22, 0xF2, 0x05])[3..],
        &[0x00, 0x07, 0xFF, 0xF2, 0x00, 0x00]
    );
    assert_eq!(
        &h.req(&[0x22, 0xF2, 0x06])[3..],
        &[0x00, 0x64, 0x00, 0xC8, 0xFE, 0xD4]
    );
}

#[test]
fn app_board_state_dids() {
    let board = FakeBoard::default();
    board.state().buttons = 0b10;
    let mut h = Harness::with_board(BootState::AppValid, board);
    assert_eq!(&h.req(&[0x22, 0xF2, 0x10])[3..], &[0x02]);
    assert_eq!(&h.req(&[0x22, 0xF2, 0x20])[3..], &[192, 168, 1, 50]);
    assert_eq!(
        &h.req(&[0x22, 0xF2, 0x21])[3..],
        &[0xC8, 0x93, 0x46, 1, 2, 3]
    );
    assert_eq!(&h.req(&[0x22, 0xF2, 0x30])[3..], &42u32.to_be_bytes());
    assert_eq!(&h.req(&[0x22, 0xF2, 0x00])[3..], &1210i32.to_be_bytes());
}

#[test]
fn broken_sensor_gives_nrc_and_dtc() {
    let board = FakeBoard::default();
    board.state().broken.push(Sensor::Pressure);
    let mut h = Harness::with_board(BootState::AppValid, board);
    assert_eq!(h.req(&[0x22, 0xF2, 0x03]), [0x7F, 0x22, 0x22]);
    let dtcs = h.req(&[0x19, 0x02, 0x08]);
    assert_eq!(
        dtcs,
        [0x59, 0x02, 0xFF, 0x01, 0xE2, 0x40, 0x2F, 0xC1, 0x02, 0x00, 0x09]
    );
}

#[test]
fn app_writes_need_extended_session() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x11, 1, 2, 3])[0], 0x7F);
    assert_eq!(h.req(&[0x10, 0x03])[..2], [0x50, 0x03]);
    assert_eq!(h.req(&[0x22, 0xF1, 0x86]), [0x62, 0xF1, 0x86, 0x03]);

    assert_eq!(h.req(&[0x2E, 0xF2, 0x11, 1, 2, 3]), [0x6E, 0xF2, 0x11]);
    assert_eq!(h.board.state().rgb, [1, 2, 3]);
    assert_eq!(&h.req(&[0x22, 0xF2, 0x11])[3..], &[1, 2, 3]);

    let text = *b"Hello AZ3166    ";
    let mut req = vec![0x2E, 0xF2, 0x12];
    req.extend_from_slice(&text);
    assert_eq!(h.req(&req), [0x6E, 0xF2, 0x12]);
    assert_eq!(h.board.state().text, text);

    let mut req = vec![0x2E, 0xF1, 0x90];
    req.extend_from_slice(b"WDB1234567890ABCD");
    assert_eq!(h.req(&req), [0x6E, 0xF1, 0x90]);
    assert_eq!(&h.req(&[0x22, 0xF1, 0x90])[3..], b"WDB1234567890ABCD");

    assert_eq!(h.req(&[0x2E, 0xF2, 0x11, 1, 2]), [0x7F, 0x2E, 0x13]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x01, 1, 2]), [0x7F, 0x2E, 0x31]);
}

#[test]
fn self_test_routine() {
    let mut h = Harness::new(BootState::AppValid);
    h.req(&[0x10, 0x03]);
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x01]),
        [0x71, 0x01, 0x10, 0x01, ROUTINE_RUNNING]
    );
    assert_eq!(h.req(&[0x31, 0x01, 0x10, 0x01]), [0x7F, 0x31, 0x24]);
    assert!(h.shared.routine_start.swap(false, Ordering::SeqCst));

    app::run_self_test(h.shared, &h.board);
    assert_eq!(h.board.state().leds, [0; board::LED_COUNT]);
    let lit: Vec<_> = h
        .board
        .state()
        .led_history
        .iter()
        .filter(|(_, d)| *d > 0)
        .copied()
        .collect();
    assert_eq!(lit, [(0, 40), (1, 55), (2, 70), (3, 85), (4, 100)]);
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x01]),
        [0x71, 0x03, 0x10, 0x01, ROUTINE_COMPLETED]
    );
    assert_eq!(h.req(&[0x31, 0x02, 0x10, 0x01]), [0x7F, 0x31, 0x24]);
}

#[test]
fn self_test_abort() {
    let mut h = Harness::new(BootState::AppValid);
    h.req(&[0x10, 0x03]);
    h.req(&[0x31, 0x01, 0x10, 0x01]);
    assert_eq!(
        h.req(&[0x31, 0x02, 0x10, 0x01]),
        [0x71, 0x02, 0x10, 0x01, ROUTINE_ABORTED]
    );
    app::run_self_test(h.shared, &h.board);
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x01]),
        [0x71, 0x03, 0x10, 0x01, ROUTINE_ABORTED]
    );
}

#[test]
fn clear_dtcs() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x19, 0x02, 0xFF]).len(), 3 + 2 * 4);
    assert_eq!(h.req(&[0x14, 0xFF, 0xFF, 0xFF]), [0x54]);
    assert_eq!(h.req(&[0x19, 0x02, 0xFF]), [0x59, 0x02, 0xFF]);
    assert_eq!(h.req(&[0x19, 0x01, 0xFF]), [0x7F, 0x19, 0x12]);
    assert_eq!(h.req(&[0x14, 0xFF]), [0x7F, 0x14, 0x13]);
}

#[test]
fn app_hard_reset_requests_bootloader() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x11, 0x01]), [0x51, 0x01]);
    assert_eq!(h.board.state().boot_state, Some(BootState::BootRequested));
    assert_eq!(h.shared.take_pending_reset(), RESET_HARD);
    assert_eq!(h.req(&[0x11, 0x03]), [0x7F, 0x11, 0x12]);
}

#[test]
fn app_has_no_security_access() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x27, 0x03])[0], 0x7F);
}

fn unlock(h: &mut Harness) {
    assert_eq!(h.req(&[0x10, 0x02])[..2], [0x50, 0x02]);
    let seed = h.req(&[0x27, 0x03]);
    assert_eq!(seed, [0x67, 0x03, 0x12, 0x34, 0x56, 0x78]);
    let key = (0x1234_5678u32 ^ boot::KEY_MASK).to_be_bytes();
    assert_eq!(
        h.req(&[0x27, 0x04, key[0], key[1], key[2], key[3]]),
        [0x67, 0x04]
    );
}

#[test]
fn boot_identification() {
    let mut h = Harness::new(BootState::BootRequested);
    assert_eq!(
        h.req(&[0x22, 0xF1, 0x00]),
        [0x62, 0xF1, 0x00, 0xFF, 0x00, 0x00]
    );
    assert_eq!(h.req(&[0x22, 0xF2, 0x01]), [0x7F, 0x22, 0x31]);
    assert_eq!(h.req(&[0x19, 0x02, 0xFF]), [0x7F, 0x19, 0x11]);
    assert_eq!(
        h.req(&[0x34, 0x00, 0x44, 0, 0, 0, 0, 0, 0, 0x10, 0]),
        [0x7F, 0x34, 0x7F]
    );
}

#[test]
fn boot_download_marks_app_valid() {
    let mut h = Harness::new(BootState::BootRequested);
    // Download needs security
    h.req(&[0x10, 0x02]);
    assert_eq!(
        h.req(&[0x34, 0x00, 0x44, 0, 0, 0, 0, 0, 0, 0x10, 0]),
        [0x7F, 0x34, 0x33]
    );

    unlock(&mut h);
    assert_eq!(
        h.req(&[0x34, 0x00, 0x44, 0, 0, 0, 0, 0, 0, 0x10, 0]),
        [0x74, 0x20, 0x0F, 0xFF]
    );
    assert_eq!(h.req(&[0x36, 0x01, 0xAA, 0xBB]), [0x76, 0x01]);
    assert_eq!(h.req(&[0x37]), [0x77]);
    assert_eq!(h.board.state().boot_state, Some(BootState::AppValid));
}

#[test]
fn boot_wrong_key() {
    let mut h = Harness::new(BootState::BootRequested);
    h.req(&[0x10, 0x02]);
    h.req(&[0x27, 0x03]);
    assert_eq!(h.req(&[0x27, 0x04, 0, 0, 0, 0]), [0x7F, 0x27, 0x35]);
}

#[test]
fn boot_resets() {
    let mut h = Harness::new(BootState::BootRequested);
    assert_eq!(h.req(&[0x11, 0x03]), [0x51, 0x03]);
    assert_eq!(h.shared.take_pending_reset(), RESET_SOFT);
    assert_eq!(h.req(&[0x11, 0x01]), [0x51, 0x01]);
    assert_eq!(h.shared.take_pending_reset(), RESET_HARD);
}

#[test]
fn tester_present_and_empty_request() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x3E, 0x00]), [0x7E, 0x00]);
    assert_eq!(h.req(&[0x3E, 0x80]), Vec::<u8>::new());
    assert_eq!(h.req(&[]), [0x7F, 0x00, 0x13]);
}

#[test]
fn s3_timeout_returns_to_default_session() {
    let mut h = Harness::new(BootState::AppValid);
    h.req(&[0x10, 0x03]);
    h.now_us += 6_000_000;
    h.ecu.tick(h.now_us);
    assert_eq!(h.req(&[0x22, 0xF1, 0x86]), [0x62, 0xF1, 0x86, 0x01]);
}

#[test]
fn ecu_size_fits_mcu_ram() {
    let size = core::mem::size_of::<Ecu<FakeBoard>>();
    std::println!("Ecu size: {size} bytes");
    assert!(size < 160 * 1024, "Ecu is {size} bytes");
}

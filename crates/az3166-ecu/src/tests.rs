/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Host tests against `docs/diagnostics.md`.

use std::sync::{Arc, Mutex};
use std::vec::Vec;

use super::*;
use crate::board::AlarmConfig;

#[derive(Default)]
struct BoardState {
    leds: [u8; board::LED_COUNT],
    led_history: Vec<(usize, u8)>,
    rgb: [u8; 3],
    text: [u8; 16],
    boot_state: Option<BootState>,
    broken: Vec<Sensor>,
    buttons: u8,
    /// Simulated app slots: (base, contents); the update target is slot B.
    flash: Vec<(u32, Vec<u8>)>,
    erased: Option<u32>,
    committed: Option<u32>,
    spoken: String,
    alarm_config: Option<AlarmConfig>,
    speaking: bool,
    volume: Option<u8>,
}

const SLOT_A: u32 = 0x0806_0000;
const SLOT_B: u32 = 0x080A_0000;
const SLOT_SIZE: usize = 0x4_0000;

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
    fn software_version(&self) -> [u8; 16] {
        *b"0.1.0\0\0\0\0\0\0\0\0\0\0\0"
    }
    fn update_begin(&self) -> Option<u32> {
        let mut s = self.state();
        s.flash = vec![
            (SLOT_A, vec![0xFF; SLOT_SIZE]),
            (SLOT_B, vec![0xFF; SLOT_SIZE]),
        ];
        s.erased = Some(SLOT_B);
        Some(SLOT_B)
    }
    fn flash_program(&self, address: u32, data: &[u8]) -> Result<(), FlashError> {
        let mut s = self.state();
        let target = s.erased.ok_or(FlashError)?;
        let slot = s
            .flash
            .iter_mut()
            .find(|(b, _)| *b == target)
            .ok_or(FlashError)?;
        let offset = address.checked_sub(target).ok_or(FlashError)? as usize;
        let cells = slot
            .1
            .get_mut(offset..offset + data.len())
            .ok_or(FlashError)?;
        if cells.iter().any(|c| *c != 0xFF) {
            return Err(FlashError); // not erased
        }
        cells.copy_from_slice(data);
        Ok(())
    }
    fn update_commit(&self, base: u32) -> Result<(), FlashError> {
        let mut s = self.state();
        s.committed = Some(base);
        s.boot_state = Some(BootState::AppValid);
        Ok(())
    }
    fn speak(&self, text: &str) -> bool {
        let mut s = self.state();
        s.spoken = text.into();
        s.speaking = true;
        true
    }
    fn speaking(&self) -> bool {
        self.state().speaking
    }
    fn speak_stop(&self) {
        self.state().speaking = false;
    }
    fn set_volume(&self, percent: u8) {
        self.state().volume = Some(percent);
    }
    fn load_alarm_config(&self) -> Option<AlarmConfig> {
        self.state().alarm_config
    }
    fn store_alarm_config(&self, config: AlarmConfig) -> Result<(), FlashError> {
        self.state().alarm_config = Some(config);
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
    assert_eq!(h.req(&[0x22, 0xF2, 0x00]), [0x7F, 0x22, 0x31]);
}

#[test]
fn broken_sensor_gives_nrc_and_dtc() {
    let board = FakeBoard::default();
    board.state().broken.push(Sensor::Pressure);
    let mut h = Harness::with_board(BootState::AppValid, board);
    assert_eq!(h.req(&[0x22, 0xF2, 0x03]), [0x7F, 0x22, 0x22]);
    let dtcs = h.req(&[0x19, 0x02, 0x08]);
    assert_eq!(dtcs, [0x59, 0x02, 0xFF, 0xC1, 0x02, 0x00, 0x09]);
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
fn self_test_needs_extended_session() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x31, 0x01, 0x10, 0x01]), [0x7F, 0x31, 0x22]);
}

#[test]
fn announce_temperature() {
    let mut h = Harness::new(BootState::AppValid);
    // Default session is enough; -12.34 C -> "minus twelve point three ..."
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x02]),
        [0x71, 0x03, 0x10, 0x02, ROUTINE_IDLE]
    );
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x02]),
        [0x71, 0x01, 0x10, 0x02, ROUTINE_RUNNING]
    );
    assert_eq!(
        h.board.state().spoken,
        "The temperature is -12.3 degrees Celsius."
    );
    assert_eq!(h.req(&[0x31, 0x01, 0x10, 0x02]), [0x7F, 0x31, 0x24]);
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x02]),
        [0x71, 0x03, 0x10, 0x02, ROUTINE_RUNNING]
    );
    h.board.state().speaking = false; // speech finished
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x02]),
        [0x71, 0x03, 0x10, 0x02, ROUTINE_COMPLETED]
    );
    // Stop after the end: acknowledged with the status, then start again
    assert_eq!(
        h.req(&[0x31, 0x02, 0x10, 0x02]),
        [0x71, 0x02, 0x10, 0x02, ROUTINE_COMPLETED]
    );
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x02]),
        [0x71, 0x01, 0x10, 0x02, ROUTINE_RUNNING]
    );
}

#[test]
fn announce_temperature_stop() {
    let mut h = Harness::new(BootState::AppValid);
    h.req(&[0x31, 0x01, 0x10, 0x02]);
    assert_eq!(
        h.req(&[0x31, 0x02, 0x10, 0x02]),
        [0x71, 0x02, 0x10, 0x02, ROUTINE_ABORTED]
    );
    assert!(!h.board.state().speaking);
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x02]),
        [0x71, 0x03, 0x10, 0x02, ROUTINE_ABORTED]
    );
}

#[test]
fn announce_needs_working_sensor() {
    let board = FakeBoard::default();
    board.state().broken.push(Sensor::HumidityTemperature);
    let mut h = Harness::with_board(BootState::AppValid, board);
    assert_eq!(h.req(&[0x31, 0x01, 0x10, 0x02]), [0x7F, 0x31, 0x22]);
}

#[test]
fn speak_text() {
    let mut h = Harness::new(BootState::AppValid);
    let mut req = vec![0x31, 0x01, 0x10, 0x05];
    req.extend_from_slice(b"Hello, world.");
    assert_eq!(h.req(&req), [0x71, 0x01, 0x10, 0x05, ROUTINE_RUNNING]);
    assert_eq!(h.board.state().spoken, "Hello, world.");
    assert_eq!(h.req(&req), [0x7F, 0x31, 0x24]);
    h.board.state().speaking = false;
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x05]),
        [0x71, 0x03, 0x10, 0x05, ROUTINE_COMPLETED]
    );
}

#[test]
fn speak_text_checks_the_text() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x31, 0x01, 0x10, 0x05]), [0x7F, 0x31, 0x13]);
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x05, b'a', 0x07]),
        [0x7F, 0x31, 0x31]
    );
    let mut req = vec![0x31, 0x01, 0x10, 0x05];
    req.extend_from_slice(&[b'a'; board::MAX_SPEECH_TEXT + 1]);
    assert_eq!(h.req(&req), [0x7F, 0x31, 0x31]);
    assert!(!h.board.state().speaking);
}

#[test]
fn speech_routines_replace_each_other() {
    let mut h = Harness::new(BootState::AppValid);
    h.req(&[0x31, 0x01, 0x10, 0x02]);
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x05, b'h', b'i']),
        [0x71, 0x01, 0x10, 0x05, ROUTINE_RUNNING]
    );
    assert_eq!(h.board.state().spoken, "hi");
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x02]),
        [0x71, 0x03, 0x10, 0x02, ROUTINE_ABORTED]
    );
    assert_eq!(
        h.req(&[0x31, 0x03, 0x10, 0x05]),
        [0x71, 0x03, 0x10, 0x05, ROUTINE_RUNNING]
    );
}

#[test]
fn volume_up_and_down() {
    let mut h = Harness::new(BootState::AppValid);
    // Starts at 100 %, default session is enough
    assert_eq!(h.req(&[0x22, 0xF2, 0x13]), [0x62, 0xF2, 0x13, 100]);
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x03]),
        [0x71, 0x01, 0x10, 0x03, 100]
    );
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x04]),
        [0x71, 0x01, 0x10, 0x04, 90]
    );
    assert_eq!(h.board.state().volume, Some(90));
    for _ in 0..12 {
        h.req(&[0x31, 0x01, 0x10, 0x04]);
    }
    assert_eq!(h.req(&[0x22, 0xF2, 0x13]), [0x62, 0xF2, 0x13, 0]);
    assert_eq!(h.board.state().volume, Some(0));
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x03]),
        [0x71, 0x01, 0x10, 0x03, 10]
    );
    // Start only: no Stop / RequestResults
    assert_eq!(h.req(&[0x31, 0x02, 0x10, 0x03]), [0x7F, 0x31, 0x12]);
}

#[test]
fn volume_write_needs_extended_and_range() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x13, 50])[..2], [0x7F, 0x2E]);
    h.req(&[0x10, 0x03]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x13, 50]), [0x6E, 0xF2, 0x13]);
    assert_eq!(h.board.state().volume, Some(50));
    assert_eq!(h.req(&[0x2E, 0xF2, 0x13, 101]), [0x7F, 0x2E, 0x31]);
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
fn healthy_board_has_no_dtcs() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x19, 0x02, 0xFF]), [0x59, 0x02, 0xFF]);
}

#[test]
fn clear_dtcs() {
    let board = FakeBoard::default();
    board
        .state()
        .broken
        .extend([Sensor::Pressure, Sensor::Magnetometer]);
    let mut h = Harness::with_board(BootState::AppValid, board);
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

fn request_download(size: usize) -> Vec<u8> {
    let mut req = vec![0x34, 0x00, 0x44];
    req.extend_from_slice(&update::APP_REGION.to_be_bytes());
    req.extend_from_slice(&(size as u32).to_be_bytes());
    req
}

/// Sends `package` in TransferData blocks of `block` data bytes.
fn transfer(h: &mut Harness, package: &[u8], block: usize) {
    for (i, chunk) in package.chunks(block).enumerate() {
        let bsc = (i + 1) as u8; // wraps 0xFF -> 0x00
        let mut req = vec![0x36, bsc];
        req.extend_from_slice(chunk);
        assert_eq!(h.req(&req), [0x76, bsc]);
    }
}

fn images() -> (Vec<u8>, Vec<u8>) {
    let a: Vec<u8> = (0..5000u32).map(|i| (i * 7) as u8).collect();
    let b: Vec<u8> = (0..5001u32).map(|i| (i * 13 + 1) as u8).collect();
    (a, b)
}

#[test]
fn boot_download_writes_target_slot_and_commits() {
    let mut h = Harness::new(BootState::BootRequested);
    let (a, b) = images();
    let package = update::build_package("0.2.0", &[(SLOT_A, &a), (SLOT_B, &b)]);

    // Download needs security
    h.req(&[0x10, 0x02]);
    assert_eq!(h.req(&request_download(package.len())), [0x7F, 0x34, 0x33]);

    unlock(&mut h);
    assert_eq!(
        h.req(&request_download(package.len())),
        [0x74, 0x20, 0x0F, 0xFF]
    );
    transfer(&mut h, &package, 1000);
    assert_eq!(h.req(&[0x37]), [0x77]);

    let state = h.board.state();
    assert_eq!(state.committed, Some(SLOT_B));
    assert_eq!(state.boot_state, Some(BootState::AppValid));
    let slot_b = &state
        .flash
        .iter()
        .find(|(base, _)| *base == SLOT_B)
        .unwrap()
        .1;
    assert_eq!(&slot_b[..b.len()], &b[..]);
    assert!(slot_b[b.len()..].iter().all(|c| *c == 0xFF));
}

#[test]
fn boot_download_tolerates_repeated_block() {
    let mut h = Harness::new(BootState::BootRequested);
    let (a, b) = images();
    let package = update::build_package("0.2.0", &[(SLOT_A, &a), (SLOT_B, &b)]);
    h.req(&[0x10, 0x02]);
    unlock(&mut h);
    h.req(&request_download(package.len()));

    let (first, rest) = package.split_at(4000);
    let mut block1 = vec![0x36, 0x01];
    block1.extend_from_slice(first);
    assert_eq!(h.req(&block1), [0x76, 0x01]);
    assert_eq!(h.req(&block1), [0x76, 0x01]); // retry: acknowledged, not rewritten
    let mut block3 = vec![0x36, 0x03];
    block3.extend_from_slice(&rest[..10]);
    assert_eq!(h.req(&block3), [0x7F, 0x36, 0x73]);
    for (i, chunk) in rest.chunks(4000).enumerate() {
        let bsc = (i + 2) as u8;
        let mut req = vec![0x36, bsc];
        req.extend_from_slice(chunk);
        assert_eq!(h.req(&req), [0x76, bsc]);
    }
    assert_eq!(h.req(&[0x37]), [0x77]);
}

#[test]
fn boot_download_rejects_corrupt_image() {
    let mut h = Harness::new(BootState::BootRequested);
    let (a, b) = images();
    let mut package = update::build_package("0.2.0", &[(SLOT_A, &a), (SLOT_B, &b)]);
    let last = package.len() - 1;
    package[last] ^= 0xFF; // inside the slot B image
    h.req(&[0x10, 0x02]);
    unlock(&mut h);
    h.req(&request_download(package.len()));
    transfer(&mut h, &package, 1000);
    assert_eq!(h.req(&[0x37]), [0x7F, 0x37, 0x72]);
    assert_eq!(h.board.state().committed, None);
}

#[test]
fn boot_download_rejects_package_without_image_for_slot() {
    let mut h = Harness::new(BootState::BootRequested);
    let (a, _) = images();
    let package = update::build_package("0.2.0", &[(SLOT_A, &a)]);
    h.req(&[0x10, 0x02]);
    unlock(&mut h);
    h.req(&request_download(package.len()));
    let mut req = vec![0x36, 0x01];
    req.extend_from_slice(&package[..200]);
    assert_eq!(h.req(&req), [0x7F, 0x36, 0x31]);
    assert_eq!(h.req(&[0x37]), [0x7F, 0x37, 0x72]);
}

#[test]
fn boot_download_rejects_wrong_address_and_size() {
    let mut h = Harness::new(BootState::BootRequested);
    h.req(&[0x10, 0x02]);
    unlock(&mut h);
    assert_eq!(
        h.req(&[0x34, 0x00, 0x44, 0, 0, 0, 0, 0, 0, 0x10, 0]),
        [0x7F, 0x34, 0x31]
    );
    assert_eq!(h.req(&request_download(10)), [0x7F, 0x34, 0x31]);
    assert_eq!(h.req(&[0x37]), [0x7F, 0x37, 0x24]);
}

#[test]
fn display_reads_match_uds_reads() {
    let mut h = Harness::new(BootState::AppValid);
    for did in [0xF190u16, 0xF201, 0xF211, 0xF212, 0xF195] {
        let mut buf = [0u8; 64];
        let n = crate::read_did(true, &h.board, h.shared, did, &mut buf).unwrap();
        let resp = h.req(&[0x22, (did >> 8) as u8, did as u8]);
        assert_eq!(&resp[3..], &buf[..n], "DID {did:04X}");
    }
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

#[test]
fn presence_and_alarm_dids() {
    use crate::presence::*;
    let mut h = Harness::new(BootState::AppValid);
    // Before the first detector step: no occupancy data
    assert_eq!(
        h.req(&[0x22, 0xF2, 0x40]),
        [0x62, 0xF2, 0x40, PRESENCE_NOT_AVAILABLE]
    );
    let mut d = Detector::new();
    d.step(&h.shared.alarm, Some(true), Some(215));
    d.step(&h.shared.alarm, Some(true), Some(245));
    assert_eq!(
        h.req(&[0x22, 0xF2, 0x40]),
        [0x62, 0xF2, 0x40, PRESENCE_OCCUPIED]
    );
    // triggered, 24.5 C, baseline 21.5 C, rise 3.0 C
    assert_eq!(
        h.req(&[0x22, 0xF2, 0x41]),
        [
            0x62,
            0xF2,
            0x41,
            ALARM_TRIGGERED,
            0x00,
            245,
            0x00,
            215,
            0x00,
            30
        ]
    );
    // ResetDetection in the default session
    assert_eq!(
        h.req(&[0x31, 0x01, 0x10, 0x06]),
        [0x71, 0x01, 0x10, 0x06, ALARM_ARMED]
    );
    assert_eq!(h.req(&[0x22, 0xF2, 0x41])[3], ALARM_ARMED);
    assert_eq!(h.req(&[0x31, 0x03, 0x10, 0x06]), [0x7F, 0x31, 0x12]);
}

#[test]
fn alarm_configuration_is_persisted() {
    let mut h = Harness::new(BootState::AppValid);
    assert_eq!(h.req(&[0x22, 0xF2, 0x42]), [0x62, 0xF2, 0x42, 0x00, 7]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x43]), [0x62, 0xF2, 0x43, 0x01, 0x2C]);
    // extended session only
    assert_eq!(h.req(&[0x2E, 0xF2, 0x42, 0x00, 10])[..2], [0x7F, 0x2E]);
    h.req(&[0x10, 0x03]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x42, 0x00, 10]), [0x6E, 0xF2, 0x42]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x43, 0x00, 60]), [0x6E, 0xF2, 0x43]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x44]), [0x62, 0xF2, 0x44, 0x00, 7]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x44, 0x00, 5]), [0x6E, 0xF2, 0x44]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x45]), [0x62, 0xF2, 0x45, 0x00, 250]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x46]), [0x62, 0xF2, 0x46, 0x00, 60]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x45, 0x01, 0x2C]), [0x6E, 0xF2, 0x45]); // 30.0 C
    assert_eq!(h.req(&[0x2E, 0xF2, 0x46, 0x00, 30]), [0x6E, 0xF2, 0x46]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x45, 0x03, 0x21]), [0x7F, 0x2E, 0x31]); // 80.1 C
    assert_eq!(h.req(&[0x2E, 0xF2, 0x46, 0x00, 0x00]), [0x7F, 0x2E, 0x31]);
    let saved = AlarmConfig {
        rise: 10,
        window_s: 60,
        fall: 5,
        hot_limit: 300,
        hot_hold_s: 30,
    };
    assert_eq!(h.board.state().alarm_config, Some(saved));
    // out of range
    assert_eq!(h.req(&[0x2E, 0xF2, 0x43, 0x00, 5]), [0x7F, 0x2E, 0x31]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x42, 0x00, 0]), [0x7F, 0x2E, 0x31]);
    assert_eq!(h.req(&[0x2E, 0xF2, 0x44, 0x01, 0xF5]), [0x7F, 0x2E, 0x31]);
    assert_eq!(h.board.state().alarm_config, Some(saved));

    // loaded at start-up
    let board = FakeBoard::default();
    // saved before F245 / F246 existed: their defaults apply
    board.state().alarm_config = Some(AlarmConfig {
        rise: 15,
        window_s: 120,
        fall: 7,
        hot_limit: board::CONFIG_UNSET,
        hot_hold_s: board::CONFIG_UNSET,
    });
    let mut h = Harness::with_board(BootState::AppValid, board);
    assert_eq!(h.req(&[0x22, 0xF2, 0x42]), [0x62, 0xF2, 0x42, 0x00, 15]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x43]), [0x62, 0xF2, 0x43, 0x00, 120]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x44]), [0x62, 0xF2, 0x44, 0x00, 7]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x45]), [0x62, 0xF2, 0x45, 0x00, 250]);
    assert_eq!(h.req(&[0x22, 0xF2, 0x46]), [0x62, 0xF2, 0x46, 0x00, 60]);
}

#[test]
fn presence_dtc_follows_the_sensor() {
    use crate::presence::*;
    let mut h = Harness::new(BootState::AppValid);
    let mut d = Detector::new();
    for _ in 0..SENSOR_FAULT_AFTER_S {
        d.step(&h.shared.alarm, None, Some(200));
    }
    assert_eq!(
        h.req(&[0x19, 0x02, 0xFF]),
        [0x59, 0x02, 0xFF, 0xC1, 0x05, 0x00, 0x09]
    );
    d.step(&h.shared.alarm, Some(false), Some(200));
    assert_eq!(
        h.req(&[0x19, 0x02, 0xFF]),
        [0x59, 0x02, 0xFF, 0xC1, 0x05, 0x00, 0x08]
    );
    assert_eq!(h.req(&[0x14, 0xFF, 0xFF, 0xFF]), [0x54]);
    assert_eq!(h.req(&[0x19, 0x02, 0xFF]), [0x59, 0x02, 0xFF]);
}

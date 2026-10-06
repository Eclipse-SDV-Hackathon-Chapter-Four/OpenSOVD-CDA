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

//! App variant: diagnostics, board sensors and actuators, self-test routine.

use core::sync::atomic::Ordering;

use ace_server::config::*;
use ace_server::handler::ServerHandler;
use ace_server::nrc::BuiltinNrc;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::UdsServer;
use ace_sim::io::NodeAddress;

use crate::board::{Board, BootState, FlashError, Sensor, LED_COUNT};
use crate::{
    common_did, Policy, Shared, ECU_ADDRESS, FUNCTIONAL_ADDRESS, RESET_HARD, ROUTINE_ABORTED,
    ROUTINE_COMPLETED, ROUTINE_RUNNING, VIN,
};

/// App variant identification: DID 0xF100 = 0x000101
pub const VARIANT_ID: [u8; 3] = [0x00, 0x01, 0x01];

const ROUTINE_SELF_TEST: u16 = 0x1001;

/// PWM duty cycles for each LED of the bar — increasing brightness.
const LED_DUTY: [u8; LED_COUNT] = [40, 55, 70, 85, 100];

/// Hardcoded simulation DTCs (from the original FLXC1000)
const DTC_FLUX_CAPACITOR_OVERLOAD: [u8; 3] = [0x01, 0xE2, 0x40];
const DTC_TEMPORAL_DISPLACEMENT: [u8; 3] = [0x03, 0x94, 0x47];

/// DTC status testFailed | confirmedDTC
const DTC_STATUS_FAILED_CONFIRMED: u8 = 0x09;

const MAX_DTCS: usize = 8;

#[derive(Clone)]
struct Dtc {
    code: [u8; 3],
    status: u8,
}

fn sensor_dtc(sensor: Sensor) -> [u8; 3] {
    match sensor {
        Sensor::HumidityTemperature => [0xC1, 0x01, 0x00],
        Sensor::Pressure => [0xC1, 0x02, 0x00],
        Sensor::Inertial => [0xC1, 0x03, 0x00],
        Sensor::Magnetometer => [0xC1, 0x04, 0x00],
    }
}

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

pub struct AppHandler<B: Board> {
    board: B,
    shared: &'static Shared,
    vin: [u8; 17],
    rgb: [u8; 3],
    display_text: [u8; 16],
    flux_capacitor_power: i32,
}

/// Converts a physical value to its raw value, rounded to nearest.
fn to_raw(value: f32, factor: f32) -> f32 {
    let raw = value / factor;
    // core has no f32::round without libm
    if raw < 0.0 {
        raw - 0.5
    } else {
        raw + 0.5
    }
}

/// Scales a physical value to a saturated big-endian i16.
fn scaled_i16(value: f32, factor: f32) -> [u8; 2] {
    // `as` truncates toward zero and saturates float-to-int conversions.
    (to_raw(value, factor) as i16).to_be_bytes()
}

fn scaled_u16(value: f32, factor: f32) -> [u8; 2] {
    (to_raw(value, factor) as u16).to_be_bytes()
}

fn write_xyz(buf: &mut [u8], xyz: [f32; 3], factor: f32) -> usize {
    for (i, v) in xyz.iter().enumerate() {
        buf[i * 2..i * 2 + 2].copy_from_slice(&scaled_i16(*v, factor));
    }
    6
}

impl<B: Board> ServerHandler for AppHandler<B> {
    type Error = BuiltinNrc;

    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, BuiltinNrc> {
        if let Some(n) = common_did(did, &VARIANT_ID, self.shared, &self.board, buf) {
            return Ok(n);
        }
        let unavailable = BuiltinNrc::ConditionsNotCorrect;
        match did {
            0xF190 => {
                buf[..17].copy_from_slice(&self.vin);
                Ok(17)
            }
            0xF200 => {
                // FluxCapacitorPowerConsumption — pulse LED1 while reading
                self.board.set_led(0, 100);
                buf[..4].copy_from_slice(&self.flux_capacitor_power.to_be_bytes());
                self.board.set_led(0, 0);
                Ok(4)
            }
            0xF201 => {
                let (temp, _) = self.board.temperature_humidity().ok_or(unavailable)?;
                buf[..2].copy_from_slice(&scaled_i16(temp, 0.1));
                Ok(2)
            }
            0xF202 => {
                let (_, humidity) = self.board.temperature_humidity().ok_or(unavailable)?;
                buf[..2].copy_from_slice(&scaled_u16(humidity, 0.1));
                Ok(2)
            }
            0xF203 => {
                let pressure = self.board.pressure_hpa().ok_or(unavailable)?;
                buf[..2].copy_from_slice(&scaled_u16(pressure, 0.1));
                Ok(2)
            }
            0xF204 => Ok(write_xyz(
                buf,
                self.board.acceleration_mg().ok_or(unavailable)?,
                1.0,
            )),
            0xF205 => {
                // mdps -> 0.1 dps
                Ok(write_xyz(
                    buf,
                    self.board.angular_rate_mdps().ok_or(unavailable)?,
                    100.0,
                ))
            }
            0xF206 => Ok(write_xyz(
                buf,
                self.board.magnetic_field_mg().ok_or(unavailable)?,
                1.0,
            )),
            0xF210 => {
                buf[0] = self.board.buttons() & 0x03;
                Ok(1)
            }
            0xF211 => {
                buf[..3].copy_from_slice(&self.rgb);
                Ok(3)
            }
            0xF212 => {
                buf[..16].copy_from_slice(&self.display_text);
                Ok(16)
            }
            0xF220 => {
                buf[..4].copy_from_slice(&self.board.ip_address());
                Ok(4)
            }
            0xF221 => {
                buf[..6].copy_from_slice(&self.board.mac_address());
                Ok(6)
            }
            0xF230 => {
                let secs = (self.board.uptime_ms() / 1000) as u32;
                buf[..4].copy_from_slice(&secs.to_be_bytes());
                Ok(4)
            }
            _ => Err(BuiltinNrc::RequestOutOfRange),
        }
    }

    fn write_did(&mut self, did: u16, data: &[u8]) -> Result<(), BuiltinNrc> {
        let expected_len = match did {
            0xF190 => 17,
            0xF211 => 3,
            0xF212 => 16,
            _ => return Err(BuiltinNrc::RequestOutOfRange),
        };
        if data.len() != expected_len {
            return Err(BuiltinNrc::IncorrectMessageLengthOrInvalidFormat);
        }
        match did {
            0xF190 => self.vin.copy_from_slice(data),
            0xF211 => {
                self.rgb.copy_from_slice(data);
                self.board.set_rgb(self.rgb);
            }
            _ => {
                self.display_text.copy_from_slice(data);
                self.board.set_display_text(&self.display_text);
            }
        }
        Ok(())
    }

    fn ecu_reset(&mut self, reset_type: u8) -> Result<(), BuiltinNrc> {
        match reset_type {
            0x01 => {
                // HardReset — switch to the bootloader
                self.board
                    .write_boot_state(BootState::BootRequested)
                    .map_err(|FlashError| BuiltinNrc::ConditionsNotCorrect)?;
                self.shared
                    .pending_reset
                    .store(RESET_HARD, Ordering::SeqCst);
                Ok(())
            }
            _ => Err(BuiltinNrc::SubFunctionNotSupported),
        }
    }

    fn routine_control(
        &mut self,
        routine_id: u16,
        sub_function: u8,
        _data: &[u8],
        buf: &mut [u8],
    ) -> Result<usize, BuiltinNrc> {
        if routine_id != ROUTINE_SELF_TEST {
            return Err(BuiltinNrc::RequestOutOfRange);
        }
        let status = &self.shared.routine_status;
        match sub_function {
            // Start — the routine task runs the LED cascade
            0x01 => {
                if status.load(Ordering::SeqCst) == ROUTINE_RUNNING {
                    return Err(BuiltinNrc::RequestSequenceError);
                }
                status.store(ROUTINE_RUNNING, Ordering::SeqCst);
                self.shared.routine_start.store(true, Ordering::SeqCst);
                buf[0] = ROUTINE_RUNNING;
                Ok(1)
            }
            // Stop — abort the running routine
            0x02 => {
                if status.load(Ordering::SeqCst) != ROUTINE_RUNNING {
                    return Err(BuiltinNrc::RequestSequenceError);
                }
                status.store(ROUTINE_ABORTED, Ordering::SeqCst);
                buf[0] = ROUTINE_ABORTED;
                Ok(1)
            }
            // RequestResults — current status
            0x03 => {
                buf[0] = status.load(Ordering::SeqCst);
                Ok(1)
            }
            _ => Err(BuiltinNrc::SubFunctionNotSupported),
        }
    }
}

// ---------------------------------------------------------------------------
// SecurityProvider — App variant has no security access
// ---------------------------------------------------------------------------

pub struct AppSecurity;

impl SecurityProvider for AppSecurity {
    fn generate_seed(&mut self, _level: u8, _buf: &mut [u8]) -> Result<usize, SecurityError> {
        Err(SecurityError::InvalidKey)
    }
    fn validate_key(&self, _level: u8, _seed: &[u8], _key: &[u8]) -> Result<(), SecurityError> {
        Err(SecurityError::InvalidKey)
    }
}

// ---------------------------------------------------------------------------
// Server config
// ---------------------------------------------------------------------------

const DEF_EXT: &[u8] = &[0x01, 0x03];
const EXT: &[u8] = &[0x03];

pub(crate) const POLICY: Policy = Policy {
    supported_sids: &[0x10, 0x11, 0x14, 0x19, 0x22, 0x2E, 0x31, 0x3E],
    reset_types: &[0x01],
    secured_sids: &[],
    secured_sessions: &[],
    security_level: 0,
};

pub fn app_server_config() -> ServerConfig {
    let mut config = ServerConfig::new(ECU_ADDRESS, FUNCTIONAL_ADDRESS)
        // Sessions
        .with_session(SessionConfig::default_session())
        .with_session(SessionConfig::extended_session())
        // Services
        .with_service(ServiceConfig::new(0x10, DEF_EXT)) // DiagnosticSessionControl
        .with_service(ServiceConfig::new(0x11, DEF_EXT)) // ECUReset
        .with_service(ServiceConfig::new(0x22, DEF_EXT)) // ReadDataByIdentifier
        .with_service(ServiceConfig::new(0x2E, EXT)) // WriteDataByIdentifier (extended only)
        .with_service(ServiceConfig::new(0x31, EXT)) // RoutineControl (extended only)
        .with_service(ServiceConfig::new(0x3E, DEF_EXT)) // TesterPresent
        // Writable DIDs (write in extended)
        .with_did(DidConfig::read_write(0xF190, DEF_EXT, EXT)) // VIN
        .with_did(DidConfig::read_write(0xF211, DEF_EXT, EXT)) // RGB LED
        .with_did(DidConfig::read_write(0xF212, DEF_EXT, EXT)); // display text

    for did in [
        0xF100, 0xF186, 0xF18C, 0xF195, // identification
        0xF200, 0xF201, 0xF202, 0xF203, 0xF204, 0xF205, 0xF206, // sensors
        0xF210, 0xF220, 0xF221, 0xF230, // board state
    ] {
        config = config.with_did(DidConfig::read_only(did, DEF_EXT));
    }
    config
}

// ---------------------------------------------------------------------------
// App ECU
// ---------------------------------------------------------------------------

pub struct AppEcu<B: Board> {
    pub(crate) server: UdsServer<AppHandler<B>, AppSecurity>,
    dtcs: heapless::Vec<Dtc, MAX_DTCS>,
}

impl<B: Board> AppEcu<B> {
    pub fn new(board: B, shared: &'static Shared) -> Self {
        let mut dtcs = heapless::Vec::new();
        let _ = dtcs.push(Dtc {
            code: DTC_FLUX_CAPACITOR_OVERLOAD,
            status: 0x2F,
        });
        let _ = dtcs.push(Dtc {
            code: DTC_TEMPORAL_DISPLACEMENT,
            status: 0x24,
        });
        for sensor in Sensor::ALL {
            if !board.sensor_ok(sensor) {
                let _ = dtcs.push(Dtc {
                    code: sensor_dtc(sensor),
                    status: DTC_STATUS_FAILED_CONFIRMED,
                });
            }
        }

        let mut display_text = [b' '; 16];
        display_text[..8].copy_from_slice(b"FLXC1000");

        let handler = AppHandler {
            board,
            shared,
            vin: *VIN,
            rgb: [0; 3],
            display_text,
            flux_capacitor_power: 1210, // 1.21 GW
        };
        Self {
            server: UdsServer::new(
                app_server_config(),
                handler,
                AppSecurity,
                NodeAddress(ECU_ADDRESS as u32),
            ),
            dtcs,
        }
    }

    /// Services the server does not implement. Returns the response length
    /// if the request was handled here.
    pub(crate) fn handle_local(&mut self, request: &[u8], response: &mut [u8]) -> Option<usize> {
        let mut out = heapless::Vec::<u8, 64>::new();
        match request[0] {
            0x14 => self.clear_dtc(request, &mut out),
            0x19 => self.read_dtc(request, &mut out),
            _ => return None,
        }
        // Responses longer than 64 bytes are truncated to what fits
        // (MAX_DTCS * 4 + 3 = 35 bytes, so this cannot happen).
        let n = out.len().min(response.len());
        response[..n].copy_from_slice(&out[..n]);
        Some(n)
    }

    fn clear_dtc(&mut self, data: &[u8], out: &mut heapless::Vec<u8, 64>) {
        if data.len() != 4 {
            let _ = out.extend_from_slice(&[0x7F, 0x14, 0x13]); // incorrectMessageLength
            return;
        }
        self.dtcs.clear();
        let _ = out.push(0x54);
    }

    fn read_dtc(&self, data: &[u8], out: &mut heapless::Vec<u8, 64>) {
        if data.len() < 2 {
            let _ = out.extend_from_slice(&[0x7F, 0x19, 0x13]);
            return;
        }
        match data[1] {
            0x02 => {
                // reportDTCByStatusMask
                if data.len() != 3 {
                    let _ = out.extend_from_slice(&[0x7F, 0x19, 0x13]);
                    return;
                }
                let status_mask = data[2];
                let _ = out.extend_from_slice(&[0x59, 0x02, 0xFF]);
                for dtc in self.dtcs.iter().filter(|d| d.status & status_mask != 0) {
                    let _ = out.extend_from_slice(&dtc.code);
                    let _ = out.push(dtc.status);
                }
            }
            _ => {
                let _ = out.extend_from_slice(&[0x7F, 0x19, 0x12]); // subFunctionNotSupported
            }
        }
    }
}

/// Self-test LED cascade: lights LEDs 1→5 one after another at increasing
/// brightness, holds all on briefly, then turns all off. Run by the routine
/// task after RoutineControl Start.
pub fn run_self_test<B: Board>(shared: &Shared, board: &B) {
    all_leds_off(board);
    for (i, duty) in LED_DUTY.iter().enumerate() {
        if shared.routine_status.load(Ordering::SeqCst) == ROUTINE_ABORTED {
            all_leds_off(board);
            return;
        }
        board.set_led(i, *duty);
        board.sleep_ms(200);
    }
    board.sleep_ms(500);
    all_leds_off(board);
    // Only complete if nobody aborted meanwhile.
    let _ = shared.routine_status.compare_exchange(
        ROUTINE_RUNNING,
        ROUTINE_COMPLETED,
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
}

/// Boot-up blink: flash the LED bar twice to signal the app is ready.
pub fn boot_blink<B: Board>(board: &B) {
    for _ in 0..2 {
        for i in 0..LED_COUNT {
            board.set_led(i, 100);
        }
        board.sleep_ms(150);
        all_leds_off(board);
        board.sleep_ms(150);
    }
}

fn all_leds_off<B: Board>(board: &B) {
    for i in 0..LED_COUNT {
        board.set_led(i, 0);
    }
}

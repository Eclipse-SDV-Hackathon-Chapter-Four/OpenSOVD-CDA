/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! `Board` implementation for the MXCHIP AZ3166 on top of the C platform.

use flxc1000_ecu::board::{Board, BootState, FlashError, Sensor};

use crate::sys;

#[derive(Clone, Copy)]
pub struct Az3166;

impl Board for Az3166 {
    fn set_led(&self, index: usize, duty_percent: u8) {
        unsafe { sys::plat_led_set(index as u32, duty_percent as u32) }
    }

    fn sleep_ms(&self, ms: u32) {
        sys::sleep_ms(ms)
    }

    fn uptime_ms(&self) -> u64 {
        sys::uptime_us() / 1000
    }

    fn random_u32(&self) -> u32 {
        unsafe { sys::plat_random() }
    }

    fn serial_number(&self) -> [u8; 12] {
        let mut uid = [0; 12];
        unsafe { sys::plat_serial_number(&mut uid) };
        uid
    }

    fn ip_address(&self) -> [u8; 4] {
        sys::net_ip()
    }

    fn mac_address(&self) -> [u8; 6] {
        sys::net_mac()
    }

    fn sensor_ok(&self, sensor: Sensor) -> bool {
        let id = match sensor {
            Sensor::HumidityTemperature => sys::SENSOR_HTS221,
            Sensor::Pressure => sys::SENSOR_LPS22HB,
            Sensor::Inertial => sys::SENSOR_LSM6DSL,
            Sensor::Magnetometer => sys::SENSOR_LIS2MDL,
        };
        unsafe { sys::plat_sensor_ok(id) != 0 }
    }

    fn temperature_humidity(&self) -> Option<(f32, f32)> {
        let (mut t, mut h) = (0.0, 0.0);
        (unsafe { sys::plat_read_hts221(&mut t, &mut h) } == 0).then_some((t, h))
    }

    fn pressure_hpa(&self) -> Option<f32> {
        let mut p = 0.0;
        (unsafe { sys::plat_read_lps22hb(&mut p) } == 0).then_some(p)
    }

    fn acceleration_mg(&self) -> Option<[f32; 3]> {
        let (mut acc, mut gyro) = ([0.0; 3], [0.0; 3]);
        (unsafe { sys::plat_read_lsm6dsl(&mut acc, &mut gyro) } == 0).then_some(acc)
    }

    fn angular_rate_mdps(&self) -> Option<[f32; 3]> {
        let (mut acc, mut gyro) = ([0.0; 3], [0.0; 3]);
        (unsafe { sys::plat_read_lsm6dsl(&mut acc, &mut gyro) } == 0).then_some(gyro)
    }

    fn magnetic_field_mg(&self) -> Option<[f32; 3]> {
        let mut mag = [0.0; 3];
        (unsafe { sys::plat_read_lis2mdl(&mut mag) } == 0).then_some(mag)
    }

    fn buttons(&self) -> u8 {
        unsafe { sys::plat_buttons() as u8 }
    }

    fn set_rgb(&self, rgb: [u8; 3]) {
        unsafe { sys::plat_rgb_set(rgb[0], rgb[1], rgb[2]) }
    }

    fn set_display_text(&self, text: &[u8; 16]) {
        sys::display_line(3, text)
    }

    fn write_boot_state(&self, state: BootState) -> Result<(), FlashError> {
        let raw = match state {
            BootState::AppValid => sys::BOOT_STATE_APP_VALID,
            BootState::BootRequested => sys::BOOT_STATE_BOOT_REQUESTED,
        };
        match unsafe { sys::plat_boot_state_write(raw) } {
            0 => Ok(()),
            _ => Err(FlashError),
        }
    }
}

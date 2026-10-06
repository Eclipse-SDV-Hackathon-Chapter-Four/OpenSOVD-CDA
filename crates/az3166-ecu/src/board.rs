/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Hardware abstraction used by the ECU logic. The firmware implements it on
//! the AZ3166; tests implement it with a fake.

/// Persistent variant selection, see `docs/diagnostics.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootState {
    /// Run the App variant.
    AppValid,
    /// Run the Boot variant.
    BootRequested,
}

/// Sensors whose initialisation result is reported as a DTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensor {
    /// HTS221 humidity / temperature
    HumidityTemperature,
    /// LPS22HB pressure
    Pressure,
    /// LSM6DSL accelerometer / gyroscope
    Inertial,
    /// LIS2MDL magnetometer
    Magnetometer,
}

impl Sensor {
    pub const ALL: [Sensor; 4] = [
        Sensor::HumidityTemperature,
        Sensor::Pressure,
        Sensor::Inertial,
        Sensor::Magnetometer,
    ];
}

/// Number of LEDs in the self-test "LED bar".
pub const LED_COUNT: usize = 5;

/// Writing the persistent boot state to flash failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashError;

pub trait Board {
    /// Sets LED `index` (0..LED_COUNT) to `duty_percent` brightness; 0 = off.
    fn set_led(&self, index: usize, duty_percent: u8);
    fn sleep_ms(&self, ms: u32);
    fn uptime_ms(&self) -> u64;
    fn random_u32(&self) -> u32;

    /// 96-bit MCU unique device ID.
    fn serial_number(&self) -> [u8; 12];
    fn ip_address(&self) -> [u8; 4];
    fn mac_address(&self) -> [u8; 6];

    fn sensor_ok(&self, sensor: Sensor) -> bool;
    /// (temperature °C, relative humidity %)
    fn temperature_humidity(&self) -> Option<(f32, f32)>;
    fn pressure_hpa(&self) -> Option<f32>;
    fn acceleration_mg(&self) -> Option<[f32; 3]>;
    fn angular_rate_mdps(&self) -> Option<[f32; 3]>;
    fn magnetic_field_mg(&self) -> Option<[f32; 3]>;

    /// Bit 0 = button A pressed, bit 1 = button B pressed.
    fn buttons(&self) -> u8;
    fn set_rgb(&self, rgb: [u8; 3]);
    fn set_display_text(&self, text: &[u8; 16]);

    fn write_boot_state(&self, state: BootState) -> Result<(), FlashError>;

    /// Firmware version, NUL padded.
    fn software_version(&self) -> [u8; 16];

    /// App update (Boot variant): erases the inactive app slot and returns
    /// its base address. Stalls the CPU for seconds.
    fn update_begin(&self) -> Option<u32>;
    /// Programs `data` at `address` inside the slot from `update_begin`.
    fn flash_program(&self, address: u32, data: &[u8]) -> Result<(), FlashError>;
    /// Marks the slot at `base` as installed (on trial) and the App to run.
    fn update_commit(&self, base: u32) -> Result<(), FlashError>;

    /// Starts speaking the word clips `words` (see `speech`) on the audio
    /// output. False if audio is unavailable.
    fn announce(&self, words: &[u8]) -> bool;
    /// True while an announcement is playing.
    fn announcing(&self) -> bool;
    /// Stops an announcement.
    fn announce_stop(&self);
    /// Sets the audio output volume, 0..=100 % (0 = mute).
    fn set_volume(&self, percent: u8);
}

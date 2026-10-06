/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Bindings to the C platform. Keep in sync with `platform/src/platform.h`.

pub const BOOT_STATE_APP_VALID: u32 = 1;
pub const BOOT_STATE_BOOT_REQUESTED: u32 = 2;

pub const SENSOR_HTS221: u32 = 0;
pub const SENSOR_LPS22HB: u32 = 1;
pub const SENSOR_LSM6DSL: u32 = 2;
pub const SENSOR_LIS2MDL: u32 = 3;

pub const TCP_SLOTS: usize = 2;

pub const TIMEOUT: i32 = 0;

extern "C" {
    pub fn plat_log(msg: *const u8, len: u32);
    pub fn plat_uptime_us() -> u64;
    pub fn plat_sleep_ms(ms: u32);
    pub fn plat_random() -> u32;
    pub fn plat_reset() -> !;

    pub fn plat_uds_execute(source: u16, req: *const u8, len: u32, resp: *mut u8, cap: u32) -> u32;

    pub fn plat_led_set(index: u32, duty_percent: u32);
    pub fn plat_rgb_set(r: u8, g: u8, b: u8);
    pub fn plat_display_line(line: u32, text: *const u8, len: u32);
    pub fn plat_display_rotate(rotated: u32);
    pub fn plat_buttons() -> u32;
    pub fn plat_serial_number(out: *mut [u8; 12]);

    pub fn plat_sensor_ok(sensor: u32) -> i32;
    pub fn plat_read_hts221(temperature_c: *mut f32, humidity_pct: *mut f32) -> i32;
    pub fn plat_read_lps22hb(pressure_hpa: *mut f32) -> i32;
    pub fn plat_read_lsm6dsl(
        acceleration_mg: *mut [f32; 3],
        angular_rate_mdps: *mut [f32; 3],
    ) -> i32;
    pub fn plat_read_lis2mdl(magnetic_mg: *mut [f32; 3]) -> i32;

    pub fn plat_boot_state_write(state: u32) -> i32;

    pub fn plat_version(out: *mut [u8; 16]);
    pub fn plat_running_slot() -> u8;
    pub fn plat_watchdog_kick();

    pub fn plat_audio_ok() -> i32;
    pub fn plat_audio_free() -> u32;
    pub fn plat_audio_pending() -> u32;
    pub fn plat_audio_write(samples: *const i16, count: u32) -> u32;
    pub fn plat_audio_clear();
    pub fn plat_audio_set_volume(percent: u32);

    pub fn plat_update_begin() -> u32;
    pub fn plat_flash_program(address: u32, data: *const u8, len: u32) -> i32;
    pub fn plat_update_commit(base: u32) -> i32;

    pub fn plat_net_mac(out: *mut [u8; 6]);
    pub fn plat_net_ip(out: *mut [u8; 4]);

    pub fn plat_udp_recv(
        buf: *mut u8,
        cap: u32,
        ip: *mut u32,
        port: *mut u16,
        timeout_ms: u32,
    ) -> i32;
    pub fn plat_udp_send(ip: u32, port: u16, buf: *const u8, len: u32) -> i32;

    pub fn plat_tcp_accept(slot: u32) -> i32;
    pub fn plat_tcp_recv(slot: u32, buf: *mut u8, cap: u32, timeout_ms: u32) -> i32;
    pub fn plat_tcp_send(slot: u32, buf: *const u8, len: u32) -> i32;
    pub fn plat_tcp_close(slot: u32);
}

/// Safe wrappers for the calls without pointer arguments.
pub fn uptime_us() -> u64 {
    unsafe { plat_uptime_us() }
}

pub fn sleep_ms(ms: u32) {
    unsafe { plat_sleep_ms(ms) }
}

pub fn display_line(line: u32, text: &[u8]) {
    unsafe { plat_display_line(line, text.as_ptr(), text.len() as u32) }
}

pub fn display_rotate(rotated: bool) {
    unsafe { plat_display_rotate(rotated as u32) }
}

pub fn net_ip() -> [u8; 4] {
    let mut ip = [0; 4];
    unsafe { plat_net_ip(&mut ip) };
    ip
}

pub fn net_mac() -> [u8; 6] {
    let mut mac = [0; 6];
    unsafe { plat_net_mac(&mut mac) };
    mac
}

pub fn version() -> [u8; 16] {
    let mut v = [0; 16];
    unsafe { plat_version(&mut v) };
    v
}

/// 'A' / 'B' for an app slot, 'L' for the bootloader.
pub fn running_slot() -> u8 {
    unsafe { plat_running_slot() }
}

pub fn watchdog_kick() {
    unsafe { plat_watchdog_kick() }
}

pub fn buttons() -> u8 {
    unsafe { plat_buttons() as u8 }
}

pub fn reset() -> ! {
    unsafe { plat_reset() }
}

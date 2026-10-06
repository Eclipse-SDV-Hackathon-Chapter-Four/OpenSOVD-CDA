/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * C <-> Rust interface. The C platform (ThreadX, NetX Duo, STM32 HAL, WICED)
 * implements plat_*; the Rust firmware (crates/az3166-fw) implements
 * az3166_*. Keep in sync with crates/az3166-fw/src/sys.rs.
 */

#ifndef AZ3166_PLATFORM_H
#define AZ3166_PLATFORM_H

#include <stdint.h>

/* Boot state (persistent variant selection) */
#define PLAT_BOOT_STATE_APP_VALID      1u
#define PLAT_BOOT_STATE_BOOT_REQUESTED 2u

/* Sensors (DTC reporting) */
#define PLAT_SENSOR_HTS221  0u
#define PLAT_SENSOR_LPS22HB 1u
#define PLAT_SENSOR_LSM6DSL 2u
#define PLAT_SENSOR_LIS2MDL 3u

/* Number of concurrent DoIP TCP connections */
#define PLAT_TCP_SLOTS 2u

/* plat_tcp_recv / plat_udp_recv results */
#define PLAT_TIMEOUT 0
#define PLAT_CLOSED  (-1)
#define PLAT_ERROR   (-2)

/* ---- implemented in C ---------------------------------------------------- */

void plat_log(const uint8_t* msg, uint32_t len);
uint64_t plat_uptime_us(void);
void plat_sleep_ms(uint32_t ms);
uint32_t plat_random(void);
void plat_reset(void) __attribute__((noreturn));

/* Runs a UDS request on the UDS worker thread (which calls
 * az3166_uds_execute) and waits for the result. Returns the response
 * length. All ECU state lives on that one thread: no locking, and only it
 * needs the large stack ace-server uses. */
uint32_t plat_uds_execute(uint16_t source, const uint8_t* req, uint32_t len, uint8_t* resp, uint32_t cap);

/* LED bar index 0..4: user LED, Azure LED, RGB red, RGB green, RGB blue. */
void plat_led_set(uint32_t index, uint32_t duty_percent);
void plat_rgb_set(uint8_t r, uint8_t g, uint8_t b);
/* OLED: line 0..3, up to 18 characters, space padded. */
void plat_display_line(uint32_t line, const uint8_t* text, uint32_t len);
/* Rotates the display content by 180 degrees (1) or back to normal (0). */
void plat_display_rotate(uint32_t rotated);
/* bit 0 = button A pressed, bit 1 = button B pressed */
uint32_t plat_buttons(void);
void plat_serial_number(uint8_t out[12]);

int32_t plat_sensor_ok(uint32_t sensor);
/* 0 = ok, otherwise the sensor is unavailable */
int32_t plat_read_hts221(float* temperature_c, float* humidity_pct);
int32_t plat_read_lps22hb(float* pressure_hpa);
int32_t plat_read_lsm6dsl(float acceleration_mg[3], float angular_rate_mdps[3]);
int32_t plat_read_lis2mdl(float magnetic_mg[3]);

/* 0 = ok */
int32_t plat_boot_state_write(uint32_t state);

/* Firmware identity: version (NUL padded) and the slot running it:
 * 'A' or 'B' for an app, 'L' for the bootloader. */
void plat_version(uint8_t out[16]);
uint8_t plat_running_slot(void);

/* Kicks the watchdog (started by the bootloader before an app). */
void plat_watchdog_kick(void);

/* Audio (app only; the bootloader has stubs). Say queues word clips (ids from
 * speech_words.h) on the headphone output: 1 = started, 0 = no audio. */
int32_t plat_audio_say(const uint8_t* words, uint32_t len);
/* 1 while playing */
int32_t plat_audio_busy(void);
void plat_audio_stop(void);
/* Volume 0..100 % (DAC digital gain, -50 dB..0 dB; 0 = mute). */
void plat_audio_set_volume(uint32_t percent);

/* App update (bootloader only). Begin erases the inactive slot (CPU stalls
 * for seconds) and returns its base address, or 0 on failure. Program
 * writes into that slot only (0 = ok, verified by read back). Commit marks
 * the slot "on trial" and the App as next to run (0 = ok). */
uint32_t plat_update_begin(void);
int32_t plat_flash_program(uint32_t address, const uint8_t* data, uint32_t len);
int32_t plat_update_commit(uint32_t base);

void plat_net_mac(uint8_t out[6]);
/* 0.0.0.0 until DHCP completes */
void plat_net_ip(uint8_t out[4]);

/* Datagram length, PLAT_TIMEOUT or PLAT_ERROR. ip/port in host order. */
int32_t plat_udp_recv(uint8_t* buf, uint32_t cap, uint32_t* ip, uint16_t* port, uint32_t timeout_ms);
int32_t plat_udp_send(uint32_t ip, uint16_t port, const uint8_t* buf, uint32_t len);

/* Blocks until a tester connects to the slot. 0 = ok. */
int32_t plat_tcp_accept(uint32_t slot);
/* Bytes received, PLAT_TIMEOUT, PLAT_CLOSED or PLAT_ERROR. */
int32_t plat_tcp_recv(uint32_t slot, uint8_t* buf, uint32_t cap, uint32_t timeout_ms);
/* 0 = ok */
int32_t plat_tcp_send(uint32_t slot, const uint8_t* buf, uint32_t len);
/* Disconnects and re-arms the slot for the next accept. */
void plat_tcp_close(uint32_t slot);

/* ---- implemented in Rust ------------------------------------------------- */

/* Builds the ECU for the given boot state. Called once, before any task. */
void az3166_init(uint32_t boot_state);
/* Called on the UDS worker thread only. */
uint32_t az3166_uds_execute(uint16_t source, const uint8_t* req, uint32_t len, uint8_t* resp, uint32_t cap);
/* Called on the UDS worker thread every 100 ms (session timeout). */
void az3166_uds_tick(void);
/* Thread bodies (never return) */
void az3166_udp_task(void);
void az3166_tcp_task(uint32_t slot);
void az3166_routine_task(void);

#endif

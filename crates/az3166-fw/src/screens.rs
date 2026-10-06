/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! OLED screens: one per readable DID (plus the network state). Button A
//! shows the previous screen, button B the next one, wrapping around. The
//! content is turned by 180 degrees when the board is held upside down
//! (accelerometer); the buttons then swap roles.
//!
//! ```text
//! line 0  App 0.2.0 A        variant, firmware version, slot
//! line 1  3/18 Pressure      screen number and title
//! line 2  973.0 hPa          value
//! line 3                     second value line, if needed
//! ```

use core::fmt::Write;
use core::sync::atomic::{AtomicU16, Ordering};

use az3166_ecu::{read_did, Shared, VOLUME_MAX, VOLUME_STEP};

use az3166_ecu::Board;

use crate::board::Az3166;
use crate::sys;

type Line = heapless::String<32>;

const REFRESH_MS: u32 = 1000;

/// Orientation: accelerometer axis that points along the display's vertical
/// and its sign (in mg) when the text is upright. The board must be tilted
/// beyond the threshold to change the orientation; lying flat keeps it.
const ORIENTATION_AXIS: usize = 1;
const UPRIGHT_SIGN: f32 = -1.0;
const ORIENTATION_THRESHOLD_MG: f32 = 500.0;
const ORIENTATION_PERIOD_MS: u32 = 300;
const NETWORK: u16 = 0x0000; // pseudo DID: network state from the board
const NO_REQUEST: u16 = 0xFFFF;

struct Screen {
    did: u16,
    title: &'static str,
    format: fn(&[u8], &mut Line, &mut Line),
}

const APP_SCREENS: &[Screen] = &[
    Screen {
        did: NETWORK,
        title: "Network",
        format: fmt_network,
    },
    Screen {
        did: 0xF201,
        title: "Temperature",
        format: fmt_temperature,
    },
    Screen {
        did: 0xF202,
        title: "Humidity",
        format: fmt_humidity,
    },
    Screen {
        did: 0xF203,
        title: "Pressure",
        format: fmt_pressure,
    },
    Screen {
        did: 0xF204,
        title: "Acceleration",
        format: fmt_acceleration,
    },
    Screen {
        did: 0xF205,
        title: "Angular rate",
        format: fmt_angular_rate,
    },
    Screen {
        did: 0xF206,
        title: "Magnetic",
        format: fmt_magnetic,
    },
    Screen {
        did: 0xF211,
        title: "RGB LED",
        format: fmt_rgb,
    },
    Screen {
        did: 0xF212,
        title: "Display text",
        format: fmt_ascii,
    },
    Screen {
        did: 0xF213,
        title: "Volume",
        format: fmt_volume,
    },
    Screen {
        did: 0xF220,
        title: "IP address",
        format: fmt_ip,
    },
    Screen {
        did: 0xF221,
        title: "MAC address",
        format: fmt_mac,
    },
    Screen {
        did: 0xF230,
        title: "Uptime",
        format: fmt_uptime,
    },
    Screen {
        did: 0xF190,
        title: "VIN",
        format: fmt_ascii,
    },
    Screen {
        did: 0xF186,
        title: "Session",
        format: fmt_session,
    },
    Screen {
        did: 0xF100,
        title: "Variant",
        format: fmt_variant,
    },
    Screen {
        did: 0xF18C,
        title: "Serial no.",
        format: fmt_serial,
    },
    Screen {
        did: 0xF195,
        title: "SW version",
        format: fmt_ascii,
    },
];

const BOOT_SCREENS: &[Screen] = &[
    Screen {
        did: NETWORK,
        title: "Network",
        format: fmt_network,
    },
    Screen {
        did: 0xF186,
        title: "Session",
        format: fmt_session,
    },
    Screen {
        did: 0xF100,
        title: "Variant",
        format: fmt_variant,
    },
    Screen {
        did: 0xF18C,
        title: "Serial no.",
        format: fmt_serial,
    },
    Screen {
        did: 0xF195,
        title: "SW version",
        format: fmt_ascii,
    },
];

/// DID whose screen should be shown next (set when a tester writes it).
static REQUESTED: AtomicU16 = AtomicU16::new(NO_REQUEST);

/// Switches to the screen of `did` at the next update.
pub fn show_did(did: u16) {
    REQUESTED.store(did, Ordering::Relaxed);
}

pub struct Ui {
    screens: &'static [Screen],
    is_app: bool,
    index: usize,
    last_buttons: u8,
    /// Per button (A, B): how long it has been held, and whether a long
    /// press already acted during this hold.
    held_ms: [u32; 2],
    long_press_fired: [bool; 2],
    next_repeat_ms: [u32; 2],
    since_refresh_ms: u32,
    since_orientation_ms: u32,
    rotated: bool,
    dirty: bool,
}

/// Long press on the Volume screen: first step after this, then repeating.
const LONG_PRESS_MS: u32 = 600;
const LONG_PRESS_REPEAT_MS: u32 = 300;
const VOLUME_DID: u16 = 0xF213;

impl Ui {
    pub fn new(is_app: bool) -> Self {
        let ui = Self {
            screens: if is_app { APP_SCREENS } else { BOOT_SCREENS },
            is_app,
            index: 0,
            last_buttons: 0,
            held_ms: [0; 2],
            long_press_fired: [false; 2],
            next_repeat_ms: [0; 2],
            since_refresh_ms: 0,
            since_orientation_ms: 0,
            rotated: false,
            dirty: true,
        };
        ui.header();
        ui
    }

    /// Call periodically (every `elapsed_ms`) from the routine task.
    pub fn update(&mut self, shared: &Shared, elapsed_ms: u32) {
        let buttons = sys::buttons();
        let pressed = buttons & !self.last_buttons;
        let released = !buttons & self.last_buttons;
        self.last_buttons = buttons;
        // A: previous, B: next. Upside down the buttons swap sides, so they
        // swap roles too: the left button always goes back.
        let (previous, next) = if self.rotated {
            (0x02, 0x01)
        } else {
            (0x01, 0x02)
        };

        if self.screens[self.index].did == VOLUME_DID {
            // Volume screen: hold = volume down / up, tap (on release) = page.
            for (bit, up) in [(previous, false), (next, true)] {
                let i = (bit >> 1) as usize; // A = 0, B = 1
                if buttons & bit != 0 {
                    if pressed & bit != 0 {
                        self.held_ms[i] = 0;
                        self.long_press_fired[i] = false;
                        self.next_repeat_ms[i] = LONG_PRESS_MS;
                    }
                    self.held_ms[i] += elapsed_ms;
                    if self.held_ms[i] >= self.next_repeat_ms[i] {
                        self.next_repeat_ms[i] = self.held_ms[i] + LONG_PRESS_REPEAT_MS;
                        self.long_press_fired[i] = true;
                        change_volume(shared, up);
                        self.dirty = true;
                    }
                } else if released & bit != 0 && !self.long_press_fired[i] {
                    self.page(up);
                }
            }
        } else {
            if pressed & previous != 0 {
                self.page(false);
            }
            if pressed & next != 0 {
                self.page(true);
            }
        }
        let requested = REQUESTED.swap(NO_REQUEST, Ordering::Relaxed);
        if let Some(i) = self.screens.iter().position(|s| s.did == requested) {
            self.index = i;
            self.dirty = true;
        }

        self.since_orientation_ms += elapsed_ms;
        if self.since_orientation_ms >= ORIENTATION_PERIOD_MS {
            self.since_orientation_ms = 0;
            self.update_orientation();
        }

        self.since_refresh_ms += elapsed_ms;
        if self.dirty || self.since_refresh_ms >= REFRESH_MS {
            self.render(shared);
            self.dirty = false;
            self.since_refresh_ms = 0;
        }
    }

    fn page(&mut self, forward: bool) {
        let n = self.screens.len();
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
        self.dirty = true;
        // A button still held on the new screen must not act as a long press.
        self.long_press_fired = [true; 2];
    }

    fn update_orientation(&mut self) {
        let Some(acceleration) = Az3166.acceleration_mg() else {
            return;
        };
        let up = acceleration[ORIENTATION_AXIS] * UPRIGHT_SIGN;
        let rotated = if up > ORIENTATION_THRESHOLD_MG {
            false
        } else if up < -ORIENTATION_THRESHOLD_MG {
            true
        } else {
            self.rotated // flat or in between: keep
        };
        if rotated != self.rotated {
            self.rotated = rotated;
            sys::display_rotate(rotated);
        }
    }

    fn header(&self) {
        let version = sys::version();
        let len = version
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(version.len());
        let mut line = Line::new();
        let _ = write!(
            line,
            "{} {}",
            if self.is_app { "App" } else { "Boot" },
            core::str::from_utf8(&version[..len]).unwrap_or("?")
        );
        if self.is_app {
            let _ = write!(line, " {}", sys::running_slot() as char);
        }
        sys::display_line(0, line.as_bytes());
    }

    fn render(&self, shared: &Shared) {
        let screen = &self.screens[self.index];
        let mut title = Line::new();
        let _ = write!(
            title,
            "{}/{} {}",
            self.index + 1,
            self.screens.len(),
            screen.title
        );

        let (mut first, mut second) = (Line::new(), Line::new());
        if screen.did == NETWORK {
            (screen.format)(&sys::net_ip(), &mut first, &mut second);
        } else {
            let mut buf = [0u8; 32];
            match read_did(self.is_app, &Az3166, shared, screen.did, &mut buf) {
                Some(n) => (screen.format)(&buf[..n], &mut first, &mut second),
                None => {
                    let _ = first.push_str("n/a");
                }
            }
        }
        sys::display_line(1, title.as_bytes());
        sys::display_line(2, first.as_bytes());
        sys::display_line(3, second.as_bytes());
    }
}

/// One volume step, like the VolumeUp / VolumeDown routines.
fn change_volume(shared: &Shared, up: bool) {
    let current = shared.volume.load(Ordering::SeqCst);
    let volume = if up {
        current.saturating_add(VOLUME_STEP).min(VOLUME_MAX)
    } else {
        current.saturating_sub(VOLUME_STEP)
    };
    shared.volume.store(volume, Ordering::SeqCst);
    Az3166.set_volume(volume);
}

// ---- value formatting (integers only: no float formatting code) ----------

fn be_i16(b: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([b[at], b[at + 1]])
}

fn be_u16(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

/// Writes `raw` x 0.1 with one decimal.
fn tenths(line: &mut Line, raw: i32) {
    let sign = if raw < 0 { "-" } else { "" };
    let abs = raw.unsigned_abs();
    let _ = write!(line, "{}{}.{}", sign, abs / 10, abs % 10);
}

fn fmt_network(ip: &[u8], first: &mut Line, second: &mut Line) {
    if ip.iter().all(|b| *b == 0) {
        let _ = first.push_str("WiFi connecting");
    } else {
        fmt_ip(ip, first, second);
        let _ = second.push_str("DoIP port 13400");
    }
}

fn fmt_temperature(b: &[u8], first: &mut Line, _: &mut Line) {
    tenths(first, be_i16(b, 0) as i32);
    let _ = first.push_str(" C");
}

fn fmt_humidity(b: &[u8], first: &mut Line, _: &mut Line) {
    tenths(first, be_u16(b, 0) as i32);
    let _ = first.push_str(" %RH");
}

fn fmt_pressure(b: &[u8], first: &mut Line, _: &mut Line) {
    tenths(first, be_u16(b, 0) as i32);
    let _ = first.push_str(" hPa");
}

fn fmt_xyz(b: &[u8], first: &mut Line, second: &mut Line, unit: &str, scaled: bool) {
    let put = |line: &mut Line, axis: &str, at: usize| {
        let _ = write!(line, "{} ", axis);
        let v = be_i16(b, at) as i32;
        if scaled {
            tenths(line, v);
        } else {
            let _ = write!(line, "{}", v);
        }
    };
    put(first, "X", 0);
    let _ = first.push_str("  ");
    put(first, "Y", 2);
    put(second, "Z", 4);
    let _ = write!(second, " {}", unit);
}

fn fmt_acceleration(b: &[u8], first: &mut Line, second: &mut Line) {
    fmt_xyz(b, first, second, "mg", false);
}

fn fmt_angular_rate(b: &[u8], first: &mut Line, second: &mut Line) {
    fmt_xyz(b, first, second, "dps", true);
}

fn fmt_magnetic(b: &[u8], first: &mut Line, second: &mut Line) {
    fmt_xyz(b, first, second, "mG", false);
}

fn fmt_rgb(b: &[u8], first: &mut Line, _: &mut Line) {
    let _ = write!(first, "R{} G{} B{}", b[0], b[1], b[2]);
}

fn fmt_ascii(b: &[u8], first: &mut Line, _: &mut Line) {
    for c in b {
        let _ = first.push(if c.is_ascii_graphic() || *c == b' ' {
            *c as char
        } else {
            ' '
        });
    }
}

fn fmt_volume(b: &[u8], first: &mut Line, second: &mut Line) {
    let percent = b[0].min(100);
    if percent == 0 {
        let _ = first.push_str("0 % (mute)");
    } else {
        let _ = write!(first, "{} %", percent);
    }
    // 10-step bar
    let _ = second.push('[');
    for i in 0..10 {
        let _ = second.push(if i < percent / 10 { '#' } else { '.' });
    }
    let _ = second.push(']');
}

fn fmt_ip(b: &[u8], first: &mut Line, _: &mut Line) {
    let _ = write!(first, "{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
}

fn fmt_mac(b: &[u8], first: &mut Line, _: &mut Line) {
    let _ = write!(
        first,
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5]
    );
}

fn fmt_uptime(b: &[u8], first: &mut Line, second: &mut Line) {
    let s = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let _ = write!(first, "{}h {:02}m {:02}s", s / 3600, s / 60 % 60, s % 60);
    let _ = write!(second, "{} s", s);
}

fn fmt_session(b: &[u8], first: &mut Line, _: &mut Line) {
    let name = match b[0] {
        0x01 => "Default",
        0x02 => "Programming",
        0x03 => "Extended",
        _ => "?",
    };
    let _ = write!(first, "{} ({:02X})", name, b[0]);
}

fn fmt_variant(b: &[u8], first: &mut Line, second: &mut Line) {
    let _ = write!(first, "{:02X} {:02X} {:02X}", b[0], b[1], b[2]);
    let _ = second.push_str(if b[0] == 0xFF { "Boot" } else { "App" });
}

fn fmt_serial(b: &[u8], first: &mut Line, second: &mut Line) {
    for byte in &b[..6] {
        let _ = write!(first, "{:02X}", byte);
    }
    for byte in &b[6..12] {
        let _ = write!(second, "{:02X}", byte);
    }
}

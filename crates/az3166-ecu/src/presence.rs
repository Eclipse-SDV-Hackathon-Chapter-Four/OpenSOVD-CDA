/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Occupancy and temperature-rise alarm.
//!
//! Once per second the firmware feeds the [`Detector`] the occupancy state
//! (from an external presence sensor) and the ambient temperature. The alarm
//! triggers when the cabin is occupied and either the temperature is at least
//! the rise threshold above the lowest temperature of the time window, or it
//! has stayed above the hot limit for the hold time (DIDs F245 / F246). It clears
//! again when the cabin is no longer occupied, when the temperature has
//! fallen by the fall threshold from its peak since the alarm triggered, or
//! by ResetDetection, which also restarts the window from the current
//! temperature.

use core::sync::atomic::{AtomicBool, AtomicI16, AtomicU16, AtomicU8, Ordering};

/// Occupancy state (DID F240).
pub const PRESENCE_CLEAR: u8 = 0;
pub const PRESENCE_OCCUPIED: u8 = 1;
pub const PRESENCE_NOT_AVAILABLE: u8 = 2;

/// Alarm state (DID F241).
pub const ALARM_ARMED: u8 = 0;
pub const ALARM_TRIGGERED: u8 = 1;

/// Rise threshold in 0.1 °C (DID F242).
pub const RISE_DEFAULT: u16 = 7;
pub const RISE_MIN: u16 = 1;
pub const RISE_MAX: u16 = 500;

/// Fall threshold in 0.1 °C (DID F244): clears a triggered alarm.
pub const FALL_DEFAULT: u16 = 7;

/// Time window in seconds (DID F243).
pub const WINDOW_DEFAULT: u16 = 300;
pub const WINDOW_MIN: u16 = 10;
pub const WINDOW_MAX: u16 = 3600;

/// Occupied and above the hot limit (0.1 °C, DID F245) for the hold time
/// (s, DID F246) in a row also triggers the alarm.
pub const HOT_LIMIT_DEFAULT: u16 = 250;
pub const HOT_LIMIT_MAX: u16 = 800;
pub const HOT_HOLD_DEFAULT: u16 = 60;
pub const HOT_HOLD_MIN: u16 = 1;
pub const HOT_HOLD_MAX: u16 = 3600;

/// Seconds without occupancy data before the sensor counts as failed (DTC).
pub const SENSOR_FAULT_AFTER_S: u16 = 10;

/// State shared between the detector (presence task) and the UDS server.
pub struct AlarmShared {
    pub presence: AtomicU8,
    pub alarm: AtomicU8,
    /// Last temperature, window minimum and their difference, in 0.1 °C.
    pub temperature: AtomicI16,
    pub baseline: AtomicI16,
    pub rise: AtomicI16,
    pub rise_threshold: AtomicU16,
    pub fall_threshold: AtomicU16,
    pub hot_limit: AtomicU16,
    pub hot_hold_s: AtomicU16,
    pub window_s: AtomicU16,
    /// Set by ResetDetection, consumed by the detector.
    pub reset_request: AtomicBool,
    /// Occupancy data missing for `SENSOR_FAULT_AFTER_S` (DTC).
    pub sensor_fault: AtomicBool,
}

impl AlarmShared {
    pub const fn new() -> Self {
        Self {
            presence: AtomicU8::new(PRESENCE_NOT_AVAILABLE),
            alarm: AtomicU8::new(ALARM_ARMED),
            temperature: AtomicI16::new(0),
            baseline: AtomicI16::new(0),
            rise: AtomicI16::new(0),
            rise_threshold: AtomicU16::new(RISE_DEFAULT),
            fall_threshold: AtomicU16::new(FALL_DEFAULT),
            hot_limit: AtomicU16::new(HOT_LIMIT_DEFAULT),
            hot_hold_s: AtomicU16::new(HOT_HOLD_DEFAULT),
            window_s: AtomicU16::new(WINDOW_DEFAULT),
            reset_request: AtomicBool::new(false),
            sensor_fault: AtomicBool::new(false),
        }
    }

    /// Clears the alarm now; the detector restarts its window on its next step.
    pub fn reset(&self) {
        self.reset_request.store(true, Ordering::SeqCst);
        self.alarm.store(ALARM_ARMED, Ordering::SeqCst);
        self.rise.store(0, Ordering::Relaxed);
    }
}

impl Default for AlarmShared {
    fn default() -> Self {
        Self::new()
    }
}

/// Temperature history (one sample per second) and the alarm decision.
pub struct Detector {
    samples: [i16; WINDOW_MAX as usize],
    /// Next write position and number of valid samples.
    head: usize,
    len: usize,
    unavailable_s: u16,
    /// Highest temperature since the alarm triggered.
    peak: i16,
    /// Seconds in a row occupied and above the hot limit.
    hot_s: u16,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    pub const fn new() -> Self {
        Self {
            samples: [0; WINDOW_MAX as usize],
            head: 0,
            len: 0,
            unavailable_s: 0,
            peak: i16::MIN,
            hot_s: 0,
        }
    }

    /// One step, once per second. `presence` is `None` if the sensor did not
    /// answer, `temperature` (0.1 °C) is `None` if the temperature sensor failed.
    pub fn step(&mut self, shared: &AlarmShared, presence: Option<bool>, temperature: Option<i16>) {
        if shared.reset_request.swap(false, Ordering::SeqCst) {
            self.len = 0;
            self.peak = i16::MIN;
            self.hot_s = 0;
        }

        let state = match presence {
            Some(true) => PRESENCE_OCCUPIED,
            Some(false) => PRESENCE_CLEAR,
            None => PRESENCE_NOT_AVAILABLE,
        };
        shared.presence.store(state, Ordering::Relaxed);
        self.unavailable_s = match presence {
            Some(_) => 0,
            None => self.unavailable_s.saturating_add(1),
        };
        shared.sensor_fault.store(
            self.unavailable_s >= SENSOR_FAULT_AFTER_S,
            Ordering::Relaxed,
        );

        let Some(temperature) = temperature else {
            return;
        };
        self.push(temperature);
        let window = shared
            .window_s
            .load(Ordering::Relaxed)
            .clamp(WINDOW_MIN, WINDOW_MAX) as usize;
        let baseline = self.window_min(window);
        let rise = temperature.saturating_sub(baseline);
        shared.temperature.store(temperature, Ordering::Relaxed);
        shared.baseline.store(baseline, Ordering::Relaxed);
        shared.rise.store(rise, Ordering::Relaxed);
        let hot_limit = shared.hot_limit.load(Ordering::Relaxed) as i16;
        self.hot_s = if state == PRESENCE_OCCUPIED && temperature > hot_limit {
            self.hot_s.saturating_add(1)
        } else {
            0
        };

        if shared.alarm.load(Ordering::SeqCst) == ALARM_TRIGGERED {
            self.peak = self.peak.max(temperature);
            let fall = shared.fall_threshold.load(Ordering::Relaxed) as i16;
            if state == PRESENCE_CLEAR || self.peak.saturating_sub(temperature) >= fall {
                self.clear(shared, temperature);
            }
            return;
        }

        let threshold = shared.rise_threshold.load(Ordering::Relaxed) as i16;
        let rose = rise >= threshold;
        let hot = self.hot_s >= shared.hot_hold_s.load(Ordering::Relaxed);
        if state == PRESENCE_OCCUPIED
            && (rose || hot)
            && !shared.reset_request.load(Ordering::SeqCst)
        {
            shared.alarm.store(ALARM_TRIGGERED, Ordering::SeqCst);
            self.peak = temperature;
        }
    }

    /// Alarm over: armed again, the window restarts from `temperature` (so
    /// the same rise does not trigger it again at once).
    fn clear(&mut self, shared: &AlarmShared, temperature: i16) {
        shared.alarm.store(ALARM_ARMED, Ordering::SeqCst);
        self.peak = i16::MIN;
        self.hot_s = 0; // the 60 s start again
        self.len = 0;
        self.push(temperature);
        shared.baseline.store(temperature, Ordering::Relaxed);
        shared.rise.store(0, Ordering::Relaxed);
    }

    fn push(&mut self, value: i16) {
        self.samples[self.head] = value;
        self.head = (self.head + 1) % self.samples.len();
        self.len = (self.len + 1).min(self.samples.len());
    }

    /// Lowest of the last `window` samples (at least one sample present).
    fn window_min(&self, window: usize) -> i16 {
        let n = window.min(self.len);
        let cap = self.samples.len();
        (1..=n)
            .map(|back| self.samples[(self.head + cap - back) % cap])
            .min()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thresholds the tests are written for: rise 3.0, fall 2.0 °C.
    fn shared() -> AlarmShared {
        let s = AlarmShared::new();
        s.rise_threshold.store(30, Ordering::SeqCst);
        s.fall_threshold.store(20, Ordering::SeqCst);
        s
    }

    fn run(d: &mut Detector, s: &AlarmShared, presence: Option<bool>, temps: &[i16]) {
        for t in temps {
            d.step(s, presence, Some(*t));
        }
    }

    #[test]
    fn triggers_on_rise_while_occupied() {
        let s = shared();
        let mut d = Detector::new();
        run(&mut d, &s, Some(true), &[220, 225, 240, 249]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[250]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        assert_eq!(s.baseline.load(Ordering::SeqCst), 220);
        assert_eq!(s.rise.load(Ordering::SeqCst), 30);
        // stays triggered while occupied and warm
        run(&mut d, &s, Some(true), &[255, 245]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        // without data it stays triggered, cleared once nobody is there
        run(&mut d, &s, None, &[255]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        run(&mut d, &s, Some(false), &[255]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        assert_eq!(s.baseline.load(Ordering::SeqCst), 255);
    }

    #[test]
    fn clears_when_the_temperature_falls_from_its_peak() {
        let s = shared();
        let mut d = Detector::new();
        run(&mut d, &s, Some(true), &[200, 230, 260, 250]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        // peak 26.0, falls 1.9: still triggered
        run(&mut d, &s, Some(true), &[241]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        // falls 2.0 (FALL_DEFAULT)
        run(&mut d, &s, Some(true), &[240]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        // armed again, from 24.0
        run(&mut d, &s, Some(true), &[260]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[270]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
    }

    #[test]
    fn needs_presence_at_the_moment_of_the_rise() {
        let s = shared();
        let mut d = Detector::new();
        run(&mut d, &s, Some(false), &[220, 260]);
        run(&mut d, &s, None, &[270]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[270]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
    }

    #[test]
    fn old_samples_leave_the_window() {
        let s = shared();
        s.window_s.store(WINDOW_MIN, Ordering::SeqCst);
        let mut d = Detector::new();
        run(&mut d, &s, Some(true), &[200]);
        run(&mut d, &s, Some(true), &[225; 10]);
        // 200 is 11 samples back: outside the 10 s window
        assert_eq!(s.baseline.load(Ordering::SeqCst), 225);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
    }

    #[test]
    fn reset_restarts_from_the_current_temperature() {
        let s = shared();
        let mut d = Detector::new();
        run(&mut d, &s, Some(true), &[200, 240]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
        s.reset();
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[240, 260]);
        assert_eq!(s.baseline.load(Ordering::SeqCst), 240);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[270]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
    }

    #[test]
    fn triggers_when_hot_for_a_minute_while_occupied() {
        let s = shared();
        let mut d = Detector::new();
        // 25.1 degC, no rise: 59 s are not enough
        run(
            &mut d,
            &s,
            Some(true),
            &[251; HOT_HOLD_DEFAULT as usize - 1],
        );
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        // nobody there for a second: the minute starts again
        run(&mut d, &s, Some(false), &[251]);
        run(
            &mut d,
            &s,
            Some(true),
            &[251; HOT_HOLD_DEFAULT as usize - 1],
        );
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[251]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
    }

    #[test]
    fn hot_limit_and_hold_time_are_configurable() {
        let s = shared();
        s.hot_limit.store(300, Ordering::SeqCst);
        s.hot_hold_s.store(5, Ordering::SeqCst);
        let mut d = Detector::new();
        run(&mut d, &s, Some(true), &[290; 10]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[301; 4]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
        run(&mut d, &s, Some(true), &[301]);
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_TRIGGERED);
    }

    #[test]
    fn exactly_the_limit_is_not_hot() {
        let s = shared();
        let mut d = Detector::new();
        run(
            &mut d,
            &s,
            Some(true),
            &[HOT_LIMIT_DEFAULT as i16; 2 * HOT_HOLD_DEFAULT as usize],
        );
        assert_eq!(s.alarm.load(Ordering::SeqCst), ALARM_ARMED);
    }

    #[test]
    fn sensor_fault_after_ten_seconds_without_data() {
        let s = shared();
        let mut d = Detector::new();
        for _ in 0..SENSOR_FAULT_AFTER_S - 1 {
            d.step(&s, None, Some(200));
        }
        assert!(!s.sensor_fault.load(Ordering::SeqCst));
        assert_eq!(s.presence.load(Ordering::SeqCst), PRESENCE_NOT_AVAILABLE);
        d.step(&s, None, Some(200));
        assert!(s.sensor_fault.load(Ordering::SeqCst));
        d.step(&s, Some(false), Some(200));
        assert!(!s.sensor_fault.load(Ordering::SeqCst));
    }
}

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

//! Cascade formant synthesizer (after Klatt, 1980), reduced to what is
//! audible at 8 kHz: a glottal pulse source and an aspiration noise source
//! through three formant resonators in series, plus a parallel frication
//! branch. Parameters are updated once per 5 ms frame and smoothed, so
//! formants glide between targets.

use core::f32::consts::PI;
use libm::{cosf, expf, sqrtf};

pub const SAMPLE_RATE: u32 = 8000;
const FS: f32 = SAMPLE_RATE as f32;
/// Samples per parameter frame (5 ms).
pub const FRAME: u32 = SAMPLE_RATE / 200;

/// Formant bandwidths (Hz).
const B1: f32 = 80.0;
const B2: f32 = 100.0;
const B3: f32 = 160.0;

/// Pitch declines from F0_START towards F0_END during a sentence.
const F0_START: f32 = 128.0;
const F0_END: f32 = 92.0;
/// Per-frame decay factor of the declination (time constant ~1.5 s).
const F0_DECAY: f32 = 0.9967;
/// Open quotient of the glottal cycle.
const OPEN_QUOTIENT: f32 = 0.6;

/// Per-frame smoothing of formants and of rising / falling amplitudes.
const SMOOTH_FORMANT: f32 = 0.3;
const SMOOTH_RISE: f32 = 0.7;
const SMOOTH_FALL: f32 = 0.45;

/// Mix levels of the sources and the output gain (tuned on the host).
const LEVEL_VOICE: f32 = 1.0;
const LEVEL_ASPIRATION: f32 = 0.12;
const LEVEL_FRICATION: f32 = 0.35;
const OUTPUT_GAIN: f32 = 2600.0;

/// Synthesizer parameters for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
    pub av: f32,
    pub ah: f32,
    pub af: f32,
    pub ff: f32,
    pub bf: f32,
}

impl Params {
    pub const SILENT: Params = Params {
        f1: 500.0,
        f2: 1500.0,
        f3: 2500.0,
        av: 0.0,
        ah: 0.0,
        af: 0.0,
        ff: 2500.0,
        bf: 1000.0,
    };
}

/// Two-pole resonator `y = a·x + b·y[n-1] + c·y[n-2]`.
#[derive(Clone, Copy)]
struct Resonator {
    a: f32,
    b: f32,
    c: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    const fn new() -> Self {
        Resonator {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Unity gain at DC (cascade formants), or unity gain at the peak
    /// (frication band-pass).
    fn tune(&mut self, freq: f32, bw: f32, unity_peak: bool) {
        let freq = freq.clamp(50.0, FS * 0.47);
        let r = expf(-PI * bw / FS);
        let theta = 2.0 * PI * freq / FS;
        self.c = -r * r;
        self.b = 2.0 * r * cosf(theta);
        self.a = if unity_peak {
            (1.0 - r) * sqrtf(1.0 - 2.0 * r * cosf(2.0 * theta) + r * r)
        } else {
            1.0 - self.b - self.c
        };
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.a * x + self.b * self.y1 + self.c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

pub struct Synth {
    cur: Params,
    r1: Resonator,
    r2: Resonator,
    r3: Resonator,
    rf: Resonator,
    f0: f32,
    /// Position in the glottal cycle, 0..1.
    phase: f32,
    /// Period length factor of the current cycle (jitter).
    jitter: f32,
    noise: u32,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub const fn new() -> Self {
        Synth {
            cur: Params::SILENT,
            r1: Resonator::new(),
            r2: Resonator::new(),
            r3: Resonator::new(),
            rf: Resonator::new(),
            f0: F0_START,
            phase: 0.0,
            jitter: 1.0,
            noise: 0x1234_5678,
        }
    }

    /// Starts a new sentence (pitch reset).
    pub fn new_sentence(&mut self) {
        self.f0 = F0_START;
    }

    /// Starts a new utterance: sentence reset and silent filters.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Moves the current parameters towards `target` and retunes the filters.
    pub fn frame(&mut self, target: &Params) {
        let c = &mut self.cur;
        let glide = |cur: &mut f32, t: f32| *cur += SMOOTH_FORMANT * (t - *cur);
        let fade = |cur: &mut f32, t: f32| {
            let k = if t > *cur { SMOOTH_RISE } else { SMOOTH_FALL };
            *cur += k * (t - *cur);
        };
        glide(&mut c.f1, target.f1);
        glide(&mut c.f2, target.f2);
        glide(&mut c.f3, target.f3);
        fade(&mut c.av, target.av);
        fade(&mut c.ah, target.ah);
        fade(&mut c.af, target.af);
        // the frication shape switches with its source
        c.ff = target.ff;
        c.bf = target.bf;

        self.r1.tune(c.f1, B1, false);
        self.r2.tune(c.f2, B2, false);
        self.r3.tune(c.f3, B3, false);
        self.rf.tune(c.ff, c.bf, true);

        self.f0 = F0_END + (self.f0 - F0_END) * F0_DECAY;
    }

    fn noise(&mut self) -> f32 {
        // xorshift32
        let mut x = self.noise;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise = x;
        (x as i32) as f32 / 2_147_483_648.0
    }

    /// Glottal flow derivative (KLGLOTT88 shape), with lip radiation.
    fn glottal(&mut self) -> f32 {
        self.phase += self.f0 * self.jitter / FS;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.jitter = 1.0 + 0.015 * self.noise();
        }
        if self.phase < OPEN_QUOTIENT {
            let t = self.phase / OPEN_QUOTIENT;
            // d/dt (t² - t³), scaled to a peak of 1
            3.0 * (2.0 * t - 3.0 * t * t)
        } else {
            0.0
        }
    }

    /// Produces the next sample.
    pub fn sample(&mut self) -> i16 {
        let glottal = self.glottal();
        let noise = self.noise();
        let c = self.cur;

        let source = LEVEL_VOICE * c.av * glottal + LEVEL_ASPIRATION * c.ah * noise;
        let voiced = self.r3.run(self.r2.run(self.r1.run(source)));
        let fric_noise = self.noise();
        let fricative = LEVEL_FRICATION * c.af * self.rf.run(fric_noise);

        let y = (voiced + fricative) * OUTPUT_GAIN;
        y.clamp(-32767.0, 32767.0) as i16
    }
}

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

//! Phonemes (ARPAbet) and their acoustic targets.
//!
//! Vowel formants follow the classic average values for adult speakers
//! (Peterson & Barney, 1952); consonant targets are approximations for a
//! simple cascade formant synthesizer running at 8 kHz.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phoneme {
    // vowels
    IY,
    IH,
    EH,
    AE,
    AA,
    AO,
    UH,
    UW,
    AH,
    AX,
    ER,
    // diphthongs
    EY,
    AY,
    OY,
    AW,
    OW,
    // approximants, nasals
    W,
    Y,
    R,
    L,
    M,
    N,
    NG,
    // fricatives
    F,
    V,
    TH,
    DH,
    S,
    Z,
    SH,
    ZH,
    HH,
    // stops, affricates
    P,
    B,
    T,
    D,
    K,
    G,
    CH,
    JH,
    // silence: short (comma, word gap) and long (sentence end)
    Pause,
    LongPause,
}

use Phoneme::*;

impl Phoneme {
    pub fn parse(name: &str) -> Option<Phoneme> {
        Some(match name {
            "IY" => IY,
            "IH" => IH,
            "EH" => EH,
            "AE" => AE,
            "AA" => AA,
            "AO" => AO,
            "UH" => UH,
            "UW" => UW,
            "AH" => AH,
            "AX" => AX,
            "ER" => ER,
            "EY" => EY,
            "AY" => AY,
            "OY" => OY,
            "AW" => AW,
            "OW" => OW,
            "W" => W,
            "Y" => Y,
            "R" => R,
            "L" => L,
            "M" => M,
            "N" => N,
            "NG" => NG,
            "F" => F,
            "V" => V,
            "TH" => TH,
            "DH" => DH,
            "S" => S,
            "Z" => Z,
            "SH" => SH,
            "ZH" => ZH,
            "HH" => HH,
            "P" => P,
            "B" => B,
            "T" => T,
            "D" => D,
            "K" => K,
            "G" => G,
            "CH" => CH,
            "JH" => JH,
            _ => return None,
        })
    }

    pub fn is_vowel(self) -> bool {
        matches!(
            self,
            IY | IH | EH | AE | AA | AO | UH | UW | AH | AX | ER | EY | AY | OY | AW | OW
        )
    }

    pub fn is_voiced(self) -> bool {
        !matches!(
            self,
            F | TH | S | SH | HH | P | T | K | CH | Pause | LongPause
        )
    }

    pub fn is_pause(self) -> bool {
        matches!(self, Pause | LongPause)
    }
}

/// Formant targets in Hz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Formants {
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
}

const fn fm(f1: f32, f2: f32, f3: f32) -> Formants {
    Formants { f1, f2, f3 }
}

/// Neutral vocal tract (schwa), used next to silence.
pub const NEUTRAL: Formants = fm(500.0, 1500.0, 2500.0);

/// One steady part of a phoneme. The synthesizer moves smoothly between the
/// targets of consecutive segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub ms: u16,
    /// Formant targets; `None` = take them from the next vocalic phoneme
    /// (aspiration, bursts), so the transition is already in place.
    pub formants: Option<Formants>,
    /// Voicing, aspiration and frication amplitude, 0..1.
    pub av: f32,
    pub ah: f32,
    pub af: f32,
    /// Frication noise resonance (Hz) and bandwidth.
    pub ff: f32,
    pub bf: f32,
}

const fn seg(
    ms: u16,
    formants: Option<Formants>,
    av: f32,
    ah: f32,
    af: f32,
    ff: f32,
    bf: f32,
) -> Segment {
    Segment {
        ms,
        formants,
        av,
        ah,
        af,
        ff,
        bf,
    }
}

const fn vowel(ms: u16, f: Formants) -> Segment {
    seg(ms, Some(f), 1.0, 0.0, 0.0, 0.0, 1.0)
}

const fn voiced(ms: u16, f: Formants, av: f32) -> Segment {
    seg(ms, Some(f), av, 0.0, 0.0, 0.0, 1.0)
}

const fn fric(ms: u16, f: Formants, av: f32, af: f32, ff: f32, bf: f32) -> Segment {
    seg(ms, Some(f), av, 0.0, af, ff, bf)
}

const fn silence(ms: u16) -> Segment {
    seg(ms, None, 0.0, 0.0, 0.0, 0.0, 1.0)
}

const fn burst(ms: u16, af: f32, ff: f32, bf: f32) -> Segment {
    seg(ms, None, 0.0, 0.0, af, ff, bf)
}

const fn aspiration(ms: u16, ah: f32) -> Segment {
    seg(ms, None, 0.0, ah, 0.0, 0.0, 1.0)
}

const fn voice_bar(ms: u16) -> Segment {
    seg(
        ms,
        Some(fm(200.0, 1100.0, 2400.0)),
        0.25,
        0.0,
        0.0,
        0.0,
        1.0,
    )
}

/// A static segment table (the `const` forces promotion of the `const fn` calls).
macro_rules! segs {
    ($($s:expr),* $(,)?) => {{
        const SEGMENTS: &[Segment] = &[$($s),*];
        SEGMENTS
    }};
}

/// The segments of a phoneme (1..=3), at normal speaking rate.
pub fn segments(p: Phoneme) -> &'static [Segment] {
    // Frication shapes (at 8 kHz the useful band ends at ~3.6 kHz).
    const S_FF: f32 = 3500.0;
    const SH_FF: f32 = 2400.0;
    const FLAT: f32 = 3000.0;
    const VOICED_F: Formants = fm(300.0, 1400.0, 2500.0);

    match p {
        IY => segs![vowel(110, fm(270.0, 2290.0, 3010.0))],
        IH => segs![vowel(80, fm(390.0, 1990.0, 2550.0))],
        EH => segs![vowel(90, fm(530.0, 1840.0, 2480.0))],
        AE => segs![vowel(120, fm(660.0, 1720.0, 2410.0))],
        AA => segs![vowel(120, fm(730.0, 1090.0, 2440.0))],
        AO => segs![vowel(120, fm(570.0, 840.0, 2410.0))],
        UH => segs![vowel(80, fm(440.0, 1020.0, 2240.0))],
        UW => segs![vowel(110, fm(300.0, 870.0, 2240.0))],
        AH => segs![vowel(80, fm(640.0, 1190.0, 2390.0))],
        AX => segs![vowel(55, fm(500.0, 1500.0, 2500.0))],
        ER => segs![vowel(110, fm(490.0, 1350.0, 1690.0))],
        EY => segs![
            vowel(90, fm(480.0, 1960.0, 2550.0)),
            vowel(70, fm(330.0, 2200.0, 2850.0))
        ],
        AY => segs![
            vowel(100, fm(710.0, 1150.0, 2450.0)),
            vowel(80, fm(400.0, 2000.0, 2600.0))
        ],
        OY => segs![
            vowel(100, fm(550.0, 850.0, 2400.0)),
            vowel(80, fm(380.0, 1950.0, 2600.0))
        ],
        AW => segs![
            vowel(100, fm(710.0, 1150.0, 2450.0)),
            vowel(80, fm(400.0, 900.0, 2300.0))
        ],
        OW => segs![
            vowel(90, fm(530.0, 900.0, 2400.0)),
            vowel(70, fm(360.0, 800.0, 2300.0))
        ],

        W => segs![voiced(60, fm(290.0, 610.0, 2150.0), 0.7)],
        Y => segs![voiced(60, fm(260.0, 2070.0, 3020.0), 0.7)],
        R => segs![voiced(60, fm(310.0, 1060.0, 1380.0), 0.7)],
        // between the light (onset) and dark (coda) L: F2 lowered
        L => segs![voiced(70, fm(310.0, 950.0, 2650.0), 0.7)],
        M => segs![voiced(70, fm(270.0, 1100.0, 2150.0), 0.45)],
        N => segs![voiced(65, fm(270.0, 1500.0, 2500.0), 0.45)],
        NG => segs![voiced(70, fm(270.0, 2000.0, 2700.0), 0.45)],

        F => segs![fric(90, VOICED_F, 0.0, 0.35, FLAT, 3000.0)],
        TH => segs![fric(80, VOICED_F, 0.0, 0.3, FLAT, 3000.0)],
        S => segs![fric(100, fm(320.0, 1500.0, 2600.0), 0.0, 1.0, S_FF, 900.0)],
        SH => segs![fric(100, fm(300.0, 1800.0, 2500.0), 0.0, 0.9, SH_FF, 900.0)],
        HH => segs![aspiration(60, 0.5)],
        V => segs![fric(60, VOICED_F, 0.45, 0.25, FLAT, 3000.0)],
        DH => segs![fric(50, VOICED_F, 0.5, 0.2, FLAT, 3000.0)],
        Z => segs![fric(80, fm(300.0, 1500.0, 2600.0), 0.4, 0.6, S_FF, 900.0)],
        ZH => segs![fric(80, fm(300.0, 1800.0, 2500.0), 0.4, 0.55, SH_FF, 900.0)],

        P => segs![
            silence(60),
            burst(10, 0.6, 900.0, 1500.0),
            aspiration(35, 0.5)
        ],
        T => segs![
            silence(55),
            burst(12, 0.9, S_FF, 1200.0),
            aspiration(35, 0.45)
        ],
        K => segs![
            silence(60),
            burst(15, 0.8, 2000.0, 1000.0),
            aspiration(40, 0.5)
        ],
        B => segs![voice_bar(50), burst(8, 0.4, 900.0, 1500.0)],
        D => segs![voice_bar(45), burst(10, 0.6, S_FF, 1200.0)],
        G => segs![voice_bar(50), burst(12, 0.5, 2000.0, 1000.0)],
        CH => segs![
            silence(50),
            fric(90, fm(300.0, 1800.0, 2500.0), 0.0, 0.9, SH_FF, 900.0)
        ],
        JH => segs![
            voice_bar(40),
            fric(70, fm(300.0, 1800.0, 2500.0), 0.35, 0.6, SH_FF, 900.0)
        ],

        Pause => segs![silence(90)],
        LongPause => segs![silence(300)],
    }
}

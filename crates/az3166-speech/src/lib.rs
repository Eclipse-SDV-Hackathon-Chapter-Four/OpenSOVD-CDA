/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! English text-to-speech for small microcontrollers, without `std` and
//! without allocation.
//!
//! Text is normalized (numbers and symbols to words), converted to phonemes
//! by a dictionary and letter-to-sound rules, and rendered by a formant
//! synthesizer as 16-bit mono PCM at 8 kHz:
//!
//! ```
//! let mut speaker = az3166_speech::Speaker::new();
//! speaker.prepare("The temperature is 23.5 degrees Celsius.");
//! let mut buf = [0i16; 256];
//! let mut total = 0;
//! loop {
//!     let n = speaker.render(&mut buf);
//!     if n == 0 { break; }
//!     total += n;
//! }
//! assert!(total > az3166_speech::SAMPLE_RATE as usize); // over a second
//! ```

#![no_std]

#[cfg(test)]
extern crate std;

mod phoneme;
mod rules;
mod synth;
mod text;

pub use phoneme::Phoneme;
pub use synth::SAMPLE_RATE;

use phoneme::{segments, Segment, NEUTRAL};
use synth::{Params, Synth, FRAME};

/// Phoneme capacity; longer texts are cut off.
pub const MAX_PHONEMES: usize = 600;

/// Speaking rate: segment durations are scaled by this factor (percent).
const DURATION_PERCENT: u32 = 117;

/// Silent frames after the last phoneme, so the filters ring out.
const TAIL_FRAMES: u16 = 8;

/// Renders one text at a time.
pub struct Speaker {
    phonemes: heapless::Vec<Phoneme, MAX_PHONEMES>,
    /// Next phoneme and segment to load.
    index: usize,
    segment: usize,
    /// Frames left in the current segment, and samples left in the frame.
    frames_left: u16,
    samples_left: u32,
    tail: u16,
    target: Params,
    synth: Synth,
}

impl Default for Speaker {
    fn default() -> Self {
        Self::new()
    }
}

impl Speaker {
    pub const fn new() -> Self {
        Speaker {
            phonemes: heapless::Vec::new(),
            index: 0,
            segment: 0,
            frames_left: 0,
            samples_left: 0,
            tail: 0,
            target: Params::SILENT,
            synth: Synth::new(),
        }
    }

    /// Converts `text` and starts rendering it from the beginning. Returns
    /// false if the text was too long and has been cut off.
    pub fn prepare(&mut self, text: &str) -> bool {
        self.phonemes.clear();
        let mut complete = true;
        text::to_phonemes(text, &mut |p| {
            if self.phonemes.push(p).is_err() {
                complete = false;
            }
        });
        self.index = 0;
        self.segment = 0;
        self.frames_left = 0;
        self.samples_left = 0;
        self.tail = TAIL_FRAMES;
        self.target = Params::SILENT;
        self.synth.reset();
        complete
    }

    /// The phonemes of the prepared text.
    pub fn phonemes(&self) -> &[Phoneme] {
        &self.phonemes
    }

    /// Stops rendering; `render` returns 0 until the next `prepare`.
    pub fn stop(&mut self) {
        self.index = self.phonemes.len();
        self.frames_left = 0;
        self.samples_left = 0;
        self.tail = 0;
    }

    /// Whether `render` has more samples.
    pub fn is_done(&self) -> bool {
        self.samples_left == 0
            && self.frames_left == 0
            && self.index >= self.phonemes.len()
            && self.tail == 0
    }

    /// Fills `out` with the next samples and returns how many were written;
    /// 0 when the text is finished.
    pub fn render(&mut self, out: &mut [i16]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.samples_left == 0 && !self.next_frame() {
                break;
            }
            out[n] = self.synth.sample();
            self.samples_left -= 1;
            n += 1;
        }
        n
    }

    /// Advances to the next 5 ms frame; false at the end.
    fn next_frame(&mut self) -> bool {
        if self.frames_left == 0 {
            if let Some(seg) = self.next_segment() {
                let ms = seg.ms as u32 * DURATION_PERCENT / 100;
                self.frames_left = ms.div_ceil(1000 * FRAME / SAMPLE_RATE).max(1) as u16;
            } else if self.tail > 0 {
                self.tail -= 1;
                self.target = Params {
                    av: 0.0,
                    ah: 0.0,
                    af: 0.0,
                    ..self.target
                };
                self.frames_left = 1;
            } else {
                return false;
            }
        }
        self.frames_left -= 1;
        self.synth.frame(&self.target);
        self.samples_left = FRAME;
        true
    }

    /// Loads the next segment as the target.
    fn next_segment(&mut self) -> Option<Segment> {
        let phoneme = *self.phonemes.get(self.index)?;
        let segs = segments(phoneme);
        let seg = segs[self.segment];
        if phoneme == Phoneme::LongPause && self.segment == 0 {
            self.synth.new_sentence();
        }
        let formants = seg.formants.unwrap_or_else(|| self.lookahead_formants());
        // Velar bursts are lower before back (rounded) vowels: "cool" vs "key".
        let (ff, bf) = if matches!(phoneme, Phoneme::K | Phoneme::G)
            && seg.af > 0.0
            && self.next_is_back_vowel()
        {
            (1200.0, 800.0)
        } else {
            (seg.ff, seg.bf)
        };
        self.target = Params {
            f1: formants.f1,
            f2: formants.f2,
            f3: formants.f3,
            av: seg.av,
            ah: seg.ah,
            af: seg.af,
            ff: if seg.af > 0.0 { ff } else { self.target.ff },
            bf: if seg.af > 0.0 { bf } else { self.target.bf },
        };

        self.segment += 1;
        if self.segment == segs.len() {
            self.segment = 0;
            self.index += 1;
        }
        Some(seg)
    }

    /// Whether the phoneme after the current one is a back vowel.
    fn next_is_back_vowel(&self) -> bool {
        use Phoneme::*;
        matches!(
            self.phonemes.get(self.index + 1),
            Some(UW | UH | OW | AO | AA | AH | W)
        )
    }

    /// Formants of the next phoneme that has its own (closures, bursts and
    /// aspiration anticipate the following vowel).
    fn lookahead_formants(&self) -> phoneme::Formants {
        for &p in &self.phonemes[self.index + 1..] {
            if p.is_pause() {
                break;
            }
            if let Some(f) = segments(p).iter().find_map(|s| s.formants) {
                return f;
            }
        }
        NEUTRAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_all(text: &str) -> std::vec::Vec<i16> {
        let mut s = Speaker::new();
        assert!(s.prepare(text));
        let mut all = std::vec::Vec::new();
        let mut buf = [0i16; 100];
        loop {
            let n = s.render(&mut buf);
            if n == 0 {
                break;
            }
            all.extend_from_slice(&buf[..n]);
        }
        all
    }

    #[test]
    fn renders_audible_speech() {
        let pcm = render_all("Hello world.");
        let secs = pcm.len() as f32 / SAMPLE_RATE as f32;
        assert!((0.5..2.5).contains(&secs), "{secs} s");
        let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 8000, "peak {peak}");
    }

    #[test]
    fn empty_text_is_silent() {
        assert!(render_all("").len() <= (TAIL_FRAMES as usize) * FRAME as usize);
    }

    #[test]
    fn stop_ends_rendering() {
        let mut s = Speaker::new();
        s.prepare("A long sentence to stop early.");
        let mut buf = [0i16; 100];
        assert_eq!(s.render(&mut buf), 100);
        s.stop();
        assert!(s.is_done());
        assert_eq!(s.render(&mut buf), 0);
    }

    #[test]
    fn long_text_is_cut_off() {
        let mut s = Speaker::new();
        let text = "WWW ".repeat(100);
        assert!(!s.prepare(&text));
        assert_eq!(s.phonemes().len(), MAX_PHONEMES);
    }
}

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

//! Speech output: the UDS worker posts a text, the speech thread renders it
//! with `az3166-speech` into the platform's audio ring buffer.

use core::sync::atomic::{AtomicU32, Ordering};

use az3166_ecu::board::MAX_SPEECH_TEXT;
use az3166_ecu::SeqBytes;
use az3166_speech::Speaker;

use crate::sys;

/// The text: length byte, then the bytes. Written by the UDS worker only.
static TEXT: SeqBytes<{ MAX_SPEECH_TEXT + 1 }> = SeqBytes::new();
/// Number of the latest request; a new one replaces the one being spoken.
static REQUESTED: AtomicU32 = AtomicU32::new(0);
/// Latest request stopped by `stop`.
static CANCELLED: AtomicU32 = AtomicU32::new(0);
/// Latest request finished (spoken, stopped or replaced) by the thread.
static DONE: AtomicU32 = AtomicU32::new(0);

/// Samples rendered per step (16 ms).
const CHUNK: usize = 128;
const IDLE_POLL_MS: u32 = 20;
const FULL_POLL_MS: u32 = 10;

/// Posts `text` (UDS worker only). False if it is too long.
pub fn request(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() > MAX_SPEECH_TEXT {
        return false;
    }
    let mut record = [0u8; MAX_SPEECH_TEXT + 1];
    record[0] = bytes.len() as u8;
    record[1..=bytes.len()].copy_from_slice(bytes);
    TEXT.set(&record);
    REQUESTED.fetch_add(1, Ordering::AcqRel);
    true
}

/// True until the latest request is finished or stopped.
pub fn busy() -> bool {
    let requested = REQUESTED.load(Ordering::Acquire);
    requested != DONE.load(Ordering::Acquire) && requested != CANCELLED.load(Ordering::Acquire)
}

/// Stops the latest request.
pub fn stop() {
    CANCELLED.store(REQUESTED.load(Ordering::Acquire), Ordering::Release);
}

/// Speech thread (low priority, App only): renders requests in real time.
#[no_mangle]
pub extern "C" fn az3166_speech_task() -> ! {
    let mut speaker = Speaker::new();
    let mut samples = [0i16; CHUNK];
    loop {
        let id = REQUESTED.load(Ordering::Acquire);
        if id == DONE.load(Ordering::Relaxed) {
            sys::sleep_ms(IDLE_POLL_MS);
            continue;
        }
        let record = TEXT.get();
        let text = &record[1..=(record[0] as usize).min(MAX_SPEECH_TEXT)];
        let text = core::str::from_utf8(text).unwrap_or("");
        if !speaker.prepare(text) {
            log!("Speech: text cut off");
        }
        log!(
            "Speech: \"{}\" ({} phonemes)",
            text,
            speaker.phonemes().len()
        );
        speak(&mut speaker, &mut samples, id);
        DONE.store(id, Ordering::Release);
    }
}

/// Renders request `id` until it ends, is stopped or replaced.
fn speak(speaker: &mut Speaker, samples: &mut [i16; CHUNK], id: u32) {
    let interrupted =
        || REQUESTED.load(Ordering::Acquire) != id || CANCELLED.load(Ordering::Acquire) == id;
    render(speaker, samples, &interrupted);
}

/// Renders the prepared text into the audio ring until it ends or
/// `interrupted` returns true.
fn render(speaker: &mut Speaker, samples: &mut [i16; CHUNK], interrupted: &dyn Fn() -> bool) {
    let mut pending = 0; // rendered, not yet queued
    loop {
        if interrupted() {
            // SAFETY: the speech thread is the only producer.
            unsafe { sys::plat_audio_clear() };
            return;
        }
        if pending == 0 {
            pending = speaker.render(samples);
            if pending == 0 {
                break;
            }
        }
        // SAFETY: `samples` holds `pending` initialized samples.
        let written = unsafe { sys::plat_audio_write(samples.as_ptr(), pending as u32) } as usize;
        samples.copy_within(written..pending, 0);
        pending -= written;
        if pending > 0 {
            sys::sleep_ms(FULL_POLL_MS);
        }
    }
    // Busy until the queued audio has been played.
    while unsafe { sys::plat_audio_pending() } > 0 {
        if interrupted() {
            unsafe { sys::plat_audio_clear() };
            return;
        }
        sys::sleep_ms(FULL_POLL_MS);
    }
}

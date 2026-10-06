/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Renders text to a WAV file on the host, to listen to the synthesizer:
//!
//! ```sh
//! cargo run -p az3166-speech --example say -- "Hello world." hello.wav
//! afplay hello.wav
//! ```

use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let text = args
        .next()
        .unwrap_or_else(|| "The temperature is 23.5 degrees Celsius.".into());
    let path = args.next().unwrap_or_else(|| "say.wav".into());

    let mut speaker = az3166_speech::Speaker::new();
    if !speaker.prepare(&text) {
        eprintln!("text cut off at {} phonemes", az3166_speech::MAX_PHONEMES);
    }
    eprintln!("{:?}", speaker.phonemes());

    let mut pcm = Vec::new();
    let mut buf = [0i16; 512];
    loop {
        let n = speaker.render(&mut buf);
        if n == 0 {
            break;
        }
        pcm.extend_from_slice(&buf[..n]);
    }

    let rate = az3166_speech::SAMPLE_RATE;
    let data_len = (pcm.len() * 2) as u32;
    let mut f = std::fs::File::create(&path)?;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&1u16.to_le_bytes())?; // mono
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in &pcm {
        f.write_all(&s.to_le_bytes())?;
    }
    let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    eprintln!(
        "{path}: {:.2} s, peak {peak}",
        pcm.len() as f32 / rate as f32
    );
    Ok(())
}

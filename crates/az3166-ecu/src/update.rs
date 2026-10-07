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

//! App update package, streamed by RequestDownload / TransferData /
//! RequestTransferExit into the inactive app slot.
//!
//! The app runs from flash at its link address, so the package carries the
//! app linked for each slot; only the image for the target slot is written.
//!
//! ```text
//! offset  size  field (little endian)
//!      0     8  magic "AZ3166UP"
//!      8    16  version, NUL padded
//!     24     4  entry count (1..=4)
//!     28  16*4  entries: link base, offset in package, size, CRC-32
//!     92     4  CRC-32 of bytes 0..92
//!     96     -  images
//! ```

use crate::board::{Board, FlashError};

pub const MAGIC: &[u8; 8] = b"AZ3166UP";
pub const HEADER_SIZE: usize = 96;
pub const MAX_ENTRIES: usize = 4;
const ENTRIES_OFFSET: usize = 28;
const ENTRY_SIZE: usize = 16;
const HEADER_CRC_OFFSET: usize = 92;

/// Address the tester sends in RequestDownload: start of the app area.
pub const APP_REGION: u32 = 0x0806_0000;
/// Largest accepted package (both slots plus header).
pub const MAX_PACKAGE_SIZE: u32 = 2 * 256 * 1024 + HEADER_SIZE as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    /// Header magic, CRC or entry table invalid.
    BadPackage,
    /// No image in the package for the slot being updated.
    NoImageForSlot,
    /// More data than announced in RequestDownload.
    TooMuchData,
    /// Download incomplete or image CRC mismatch at transfer exit.
    Verification,
    Flash,
}

impl From<FlashError> for UpdateError {
    fn from(_: FlashError) -> Self {
        UpdateError::Flash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub link_base: u32,
    pub offset: u32,
    pub size: u32,
    pub crc: u32,
}

/// CRC-32 (IEEE 802.3, as zlib / `binascii.crc32`).
#[derive(Clone, Copy)]
pub struct Crc32(u32);

impl Crc32 {
    pub const fn new() -> Self {
        Crc32(0xFFFF_FFFF)
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut crc = self.0;
        for byte in data {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        self.0 = crc;
    }

    pub fn finish(self) -> u32 {
        !self.0
    }

    pub fn of(data: &[u8]) -> u32 {
        let mut crc = Crc32::new();
        crc.update(data);
        crc.finish()
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Parses and validates a package header; returns the version and the entry
/// for `target`.
pub fn parse_header(header: &[u8], target: u32) -> Result<([u8; 16], Entry), UpdateError> {
    if header.len() < HEADER_SIZE || &header[..8] != MAGIC {
        return Err(UpdateError::BadPackage);
    }
    if Crc32::of(&header[..HEADER_CRC_OFFSET]) != u32_at(header, HEADER_CRC_OFFSET) {
        return Err(UpdateError::BadPackage);
    }
    let count = u32_at(header, 24) as usize;
    if count == 0 || count > MAX_ENTRIES {
        return Err(UpdateError::BadPackage);
    }
    let mut version = [0u8; 16];
    version.copy_from_slice(&header[8..24]);

    (0..count)
        .map(|i| {
            let at = ENTRIES_OFFSET + i * ENTRY_SIZE;
            Entry {
                link_base: u32_at(header, at),
                offset: u32_at(header, at + 4),
                size: u32_at(header, at + 8),
                crc: u32_at(header, at + 12),
            }
        })
        .find(|e| e.link_base == target)
        .filter(|e| e.offset as usize >= HEADER_SIZE && e.size > 0)
        .map(|e| (version, e))
        .ok_or(UpdateError::NoImageForSlot)
}

/// An active download into slot `target`.
pub struct Download {
    target: u32,
    total: u32,
    received: u32,
    header: heapless::Vec<u8, HEADER_SIZE>,
    entry: Option<Entry>,
    crc: Crc32,
    written: u32,
    last_block: Option<u8>,
}

/// What to do with a TransferData block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// Next block in sequence: write it.
    Next,
    /// Repeat of the previous block (tester retry): acknowledge only.
    Repeat,
    /// Out of sequence.
    Wrong,
}

impl Download {
    pub fn new(target: u32, total: u32) -> Self {
        Self {
            target,
            total,
            received: 0,
            header: heapless::Vec::new(),
            entry: None,
            crc: Crc32::new(),
            written: 0,
            last_block: None,
        }
    }

    pub fn target(&self) -> u32 {
        self.target
    }

    /// Classifies block sequence counter `bsc` (starts at 1, wraps 0xFF -> 0x00).
    pub fn check_block(&self, bsc: u8) -> Block {
        match self.last_block {
            None if bsc == 1 => Block::Next,
            None => Block::Wrong,
            Some(last) if bsc == last => Block::Repeat,
            Some(last) if bsc == last.wrapping_add(1) => Block::Next,
            Some(_) => Block::Wrong,
        }
    }

    /// Writes one in-sequence block.
    pub fn write_block<B: Board>(
        &mut self,
        bsc: u8,
        data: &[u8],
        board: &B,
    ) -> Result<(), UpdateError> {
        if self.received as usize + data.len() > self.total as usize {
            return Err(UpdateError::TooMuchData);
        }
        let mut position = self.received;
        let mut rest = data;

        // Header first.
        if self.header.len() < HEADER_SIZE {
            let take = (HEADER_SIZE - self.header.len()).min(rest.len());
            let _ = self.header.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            position += take as u32;
            if self.header.len() == HEADER_SIZE {
                let (_, entry) = parse_header(&self.header, self.target)?;
                if entry.size > MAX_PACKAGE_SIZE
                    || entry.offset.saturating_add(entry.size) > self.total
                {
                    return Err(UpdateError::BadPackage);
                }
                self.entry = Some(entry);
            }
        }

        if let Some(entry) = self.entry {
            let start = position;
            let end = position + rest.len() as u32;
            let from = start.max(entry.offset);
            let to = end.min(entry.offset + entry.size);
            if from < to {
                let chunk = &rest[(from - start) as usize..(to - start) as usize];
                board.flash_program(self.target + (from - entry.offset), chunk)?;
                self.crc.update(chunk);
                self.written += chunk.len() as u32;
            }
        }

        self.received += data.len() as u32;
        self.last_block = Some(bsc);
        Ok(())
    }

    /// Checks the download is complete and the image intact.
    pub fn verify(&self) -> Result<(), UpdateError> {
        let entry = self.entry.ok_or(UpdateError::Verification)?;
        if self.received != self.total
            || self.written != entry.size
            || self.crc.finish() != entry.crc
        {
            return Err(UpdateError::Verification);
        }
        Ok(())
    }
}

/// Builds a package (host tools and tests).
#[cfg(test)]
pub fn build_package(version: &str, images: &[(u32, &[u8])]) -> std::vec::Vec<u8> {
    let mut header = std::vec![0u8; HEADER_SIZE];
    header[..8].copy_from_slice(MAGIC);
    let v = version.as_bytes();
    header[8..8 + v.len().min(16)].copy_from_slice(&v[..v.len().min(16)]);
    header[24..28].copy_from_slice(&(images.len() as u32).to_le_bytes());
    let mut offset = HEADER_SIZE as u32;
    for (i, (base, image)) in images.iter().enumerate() {
        let at = ENTRIES_OFFSET + i * ENTRY_SIZE;
        header[at..at + 4].copy_from_slice(&base.to_le_bytes());
        header[at + 4..at + 8].copy_from_slice(&offset.to_le_bytes());
        header[at + 8..at + 12].copy_from_slice(&(image.len() as u32).to_le_bytes());
        header[at + 12..at + 16].copy_from_slice(&Crc32::of(image).to_le_bytes());
        offset += image.len() as u32;
    }
    let crc = Crc32::of(&header[..HEADER_CRC_OFFSET]);
    header[HEADER_CRC_OFFSET..HEADER_SIZE].copy_from_slice(&crc.to_le_bytes());
    let mut package = header;
    for (_, image) in images {
        package.extend_from_slice(image);
    }
    package
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_zlib() {
        assert_eq!(Crc32::of(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn header_selects_target_slot() {
        let package = build_package(
            "0.2.0",
            &[(0x0806_0000, &[1, 2, 3]), (0x080A_0000, &[4, 5])],
        );
        let (version, entry) = parse_header(&package, 0x080A_0000).unwrap();
        assert_eq!(&version[..5], b"0.2.0");
        assert_eq!(entry.offset as usize, HEADER_SIZE + 3);
        assert_eq!(entry.size, 2);
        assert_eq!(
            parse_header(&package, 0x0800_0000),
            Err(UpdateError::NoImageForSlot)
        );
    }

    #[test]
    fn corrupt_header_is_rejected() {
        let mut package = build_package("0.2.0", &[(0x0806_0000, &[1, 2, 3])]);
        package[30] ^= 1;
        assert_eq!(
            parse_header(&package, 0x0806_0000),
            Err(UpdateError::BadPackage)
        );
    }

    #[test]
    fn block_sequence_wraps_and_tolerates_repeats() {
        let mut d = Download::new(0, 10);
        assert_eq!(d.check_block(2), Block::Wrong);
        assert_eq!(d.check_block(1), Block::Next);
        d.last_block = Some(0xFF);
        assert_eq!(d.check_block(0x00), Block::Next);
        assert_eq!(d.check_block(0xFF), Block::Repeat);
        assert_eq!(d.check_block(0x05), Block::Wrong);
    }
}

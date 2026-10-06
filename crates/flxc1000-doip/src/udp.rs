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

//! UDP vehicle identification (port 13400).

use crate::header::{DoipHeader, PayloadType};
use crate::message::VehicleAnnouncement;

/// DoIP entity configuration
#[derive(Debug, Clone)]
pub struct DoipConfig {
    pub ecu_address: u16,
    pub vin: [u8; 17],
    pub eid: [u8; 6],
    pub gid: [u8; 6],
}

impl DoipConfig {
    pub fn new(ecu_address: u16, vin: &[u8; 17]) -> Self {
        Self {
            ecu_address,
            vin: *vin,
            eid: [0x00; 6],
            gid: [0x00; 6],
        }
    }

    pub fn with_eid(mut self, eid: [u8; 6]) -> Self {
        self.eid = eid;
        self
    }

    /// Encodes a vehicle announcement / identification response into `out`.
    pub fn announcement(&self, out: &mut [u8]) -> Option<usize> {
        VehicleAnnouncement {
            vin: self.vin,
            logical_address: self.ecu_address,
            eid: self.eid,
            gid: self.gid,
            further_action: 0x00,
            sync_status: 0x00,
        }
        .encode(out)
    }
}

/// Handles one UDP datagram. Returns the length of the reply written to
/// `out`, or `None` if the datagram needs no reply.
pub fn handle_datagram(datagram: &[u8], config: &DoipConfig, out: &mut [u8]) -> Option<usize> {
    let header = DoipHeader::parse(datagram)?;
    if !header.is_valid() {
        return None;
    }
    let payload = &datagram[DoipHeader::SIZE..];
    if payload.len() != header.payload_length as usize {
        return None;
    }

    let matches = match PayloadType::from_u16(header.payload_type)? {
        PayloadType::VehicleIdentificationRequest => payload.is_empty(),
        PayloadType::VehicleIdentificationRequestEid => payload == config.eid,
        PayloadType::VehicleIdentificationRequestVin => payload == config.vin,
        _ => false,
    };
    if matches {
        config.announcement(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> DoipConfig {
        DoipConfig::new(0x1000, b"FLXC1000TEST00001").with_eid([1, 2, 3, 4, 5, 6])
    }

    #[test]
    fn identification_request_gets_announcement() {
        let mut out = [0u8; 64];
        let n = handle_datagram(&[0xFF, 0x00, 0, 1, 0, 0, 0, 0], &config(), &mut out).unwrap();
        assert_eq!(n, VehicleAnnouncement::FRAME_SIZE);
        assert_eq!(&out[..4], &[0x02, 0xFD, 0x00, 0x04]);
        assert_eq!(&out[8..25], b"FLXC1000TEST00001");
        assert_eq!(&out[25..27], &[0x10, 0x00]);
        assert_eq!(&out[27..33], &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn request_by_eid_only_matches_own_eid() {
        let mut out = [0u8; 64];
        let mut req = std::vec![0x02, 0xFD, 0, 2, 0, 0, 0, 6];
        req.extend([1, 2, 3, 4, 5, 6]);
        assert!(handle_datagram(&req, &config(), &mut out).is_some());
        req[13] = 7;
        assert!(handle_datagram(&req, &config(), &mut out).is_none());
    }

    #[test]
    fn request_by_vin_only_matches_own_vin() {
        let mut out = [0u8; 64];
        let mut req = std::vec![0x02, 0xFD, 0, 3, 0, 0, 0, 17];
        req.extend(b"FLXC1000TEST00001");
        assert!(handle_datagram(&req, &config(), &mut out).is_some());
        req[24] = b'2';
        assert!(handle_datagram(&req, &config(), &mut out).is_none());
    }

    #[test]
    fn invalid_header_is_ignored() {
        let mut out = [0u8; 64];
        assert!(handle_datagram(&[0x02, 0x00, 0, 1, 0, 0, 0, 0], &config(), &mut out).is_none());
    }
}

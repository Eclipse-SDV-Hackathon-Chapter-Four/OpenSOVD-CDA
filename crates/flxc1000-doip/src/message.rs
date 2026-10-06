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

use crate::header::{DoipHeader, PayloadType};

/// Writes a DoIP frame (header + payload parts) into `out`.
/// Returns the frame length, or `None` if `out` is too small.
fn write_frame(out: &mut [u8], payload_type: PayloadType, parts: &[&[u8]]) -> Option<usize> {
    let payload_len: usize = parts.iter().map(|p| p.len()).sum();
    let total = DoipHeader::SIZE + payload_len;
    if out.len() < total {
        return None;
    }
    out[..DoipHeader::SIZE]
        .copy_from_slice(&DoipHeader::new(payload_type, payload_len as u32).encode());
    let mut offset = DoipHeader::SIZE;
    for part in parts {
        out[offset..offset + part.len()].copy_from_slice(part);
        offset += part.len();
    }
    Some(total)
}

/// Vehicle Announcement / Identification Response Message
#[derive(Debug, Clone)]
pub struct VehicleAnnouncement {
    pub vin: [u8; 17],
    pub logical_address: u16,
    pub eid: [u8; 6],       // entity ID (MAC address)
    pub gid: [u8; 6],       // group ID
    pub further_action: u8, // 0x00 = no further action required
    pub sync_status: u8,    // 0x00 = synchronized
}

impl VehicleAnnouncement {
    pub const FRAME_SIZE: usize = DoipHeader::SIZE + 33;

    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::VehicleAnnouncementMessage,
            &[
                &self.vin,
                &self.logical_address.to_be_bytes(),
                &self.eid,
                &self.gid,
                &[self.further_action, self.sync_status],
            ],
        )
    }
}

/// Routing Activation Request (received from tester)
#[derive(Debug, Clone)]
pub struct RoutingActivationRequest {
    pub source_address: u16,
    pub activation_type: u8,
    pub reserved: u32,
}

impl RoutingActivationRequest {
    pub fn parse(payload: &[u8]) -> Option<Self> {
        if payload.len() < 7 {
            return None;
        }
        Some(Self {
            source_address: u16::from_be_bytes([payload[0], payload[1]]),
            activation_type: payload[2],
            reserved: u32::from_be_bytes([payload[3], payload[4], payload[5], payload[6]]),
        })
    }
}

/// Routing Activation Response (sent to tester)
#[derive(Debug, Clone)]
pub struct RoutingActivationResponse {
    pub tester_address: u16,
    pub entity_address: u16,
    pub response_code: u8,
    pub reserved: u32,
}

/// Routing activation response codes
pub mod routing_response_code {
    pub const SUCCESS: u8 = 0x10;
    pub const DENIED_UNKNOWN_SA: u8 = 0x00;
    pub const DENIED_ALL_SOCKETS_ACTIVE: u8 = 0x01;
    pub const DENIED_SA_DIFFERENT: u8 = 0x02;
    pub const DENIED_SA_ALREADY_ACTIVE: u8 = 0x03;
}

impl RoutingActivationResponse {
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::RoutingActivationResponse,
            &[
                &self.tester_address.to_be_bytes(),
                &self.entity_address.to_be_bytes(),
                &[self.response_code],
                &self.reserved.to_be_bytes(),
            ],
        )
    }
}

/// Diagnostic Message (received from tester or sent as response)
#[derive(Debug, Clone)]
pub struct DiagnosticMessage<'a> {
    pub source_address: u16,
    pub target_address: u16,
    pub user_data: &'a [u8],
}

impl<'a> DiagnosticMessage<'a> {
    pub fn parse(payload: &'a [u8]) -> Option<Self> {
        if payload.len() < 4 {
            return None;
        }
        Some(Self {
            source_address: u16::from_be_bytes([payload[0], payload[1]]),
            target_address: u16::from_be_bytes([payload[2], payload[3]]),
            user_data: &payload[4..],
        })
    }

    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::DiagnosticMessage,
            &[
                &self.source_address.to_be_bytes(),
                &self.target_address.to_be_bytes(),
                self.user_data,
            ],
        )
    }
}

/// Diagnostic Message Positive Acknowledgement
#[derive(Debug, Clone)]
pub struct DiagnosticMessageAck {
    pub source_address: u16,
    pub target_address: u16,
    pub ack_code: u8,
}

impl DiagnosticMessageAck {
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::DiagnosticMessagePositiveAck,
            &[
                &self.source_address.to_be_bytes(),
                &self.target_address.to_be_bytes(),
                &[self.ack_code],
            ],
        )
    }
}

/// Diagnostic Message Negative Acknowledgement
#[derive(Debug, Clone)]
pub struct DiagnosticMessageNack {
    pub source_address: u16,
    pub target_address: u16,
    pub nack_code: u8,
}

/// NACK codes
pub mod diag_nack_code {
    pub const INVALID_SOURCE_ADDRESS: u8 = 0x02;
    pub const UNKNOWN_TARGET_ADDRESS: u8 = 0x03;
    pub const MESSAGE_TOO_LARGE: u8 = 0x04;
    pub const OUT_OF_MEMORY: u8 = 0x05;
    pub const TARGET_UNREACHABLE: u8 = 0x06;
}

impl DiagnosticMessageNack {
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::DiagnosticMessageNegativeAck,
            &[
                &self.source_address.to_be_bytes(),
                &self.target_address.to_be_bytes(),
                &[self.nack_code],
            ],
        )
    }
}

/// Generic DoIP header negative acknowledgement
pub struct GenericNack {
    pub nack_code: u8,
}

/// Generic header NACK codes
pub mod generic_nack_code {
    pub const INCORRECT_PATTERN: u8 = 0x00;
    pub const UNKNOWN_PAYLOAD_TYPE: u8 = 0x01;
    pub const MESSAGE_TOO_LARGE: u8 = 0x02;
    pub const OUT_OF_MEMORY: u8 = 0x03;
    pub const INVALID_PAYLOAD_LENGTH: u8 = 0x04;
}

impl GenericNack {
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(out, PayloadType::GenericNack, &[&[self.nack_code]])
    }
}

/// Alive Check Response
pub struct AliveCheckResponse {
    pub source_address: u16,
}

impl AliveCheckResponse {
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        write_frame(
            out,
            PayloadType::AliveCheckResponse,
            &[&self.source_address.to_be_bytes()],
        )
    }
}

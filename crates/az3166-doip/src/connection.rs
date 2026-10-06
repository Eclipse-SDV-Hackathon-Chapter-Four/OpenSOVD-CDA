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

//! Sans-IO DoIP TCP connection: feed received bytes into
//! [`Connection::on_data`], frames to send come out through [`Transport`].

use crate::header::{DoipHeader, PayloadType};
use crate::message::*;

/// Largest UDS message (request or response) carried in a diagnostic message.
pub const MAX_UDS_SIZE: usize = 4096;

/// Largest accepted DoIP payload: diagnostic message SA + TA + UDS data.
pub const MAX_PAYLOAD_SIZE: usize = 4 + MAX_UDS_SIZE;

const FRAME_CAPACITY: usize = DoipHeader::SIZE + MAX_PAYLOAD_SIZE;

/// Offset of the UDS data inside an encoded diagnostic message frame.
const DIAG_DATA_OFFSET: usize = DoipHeader::SIZE + 4;

/// Inactivity timeout after which the transport should close the connection.
pub const INACTIVITY_TIMEOUT_MS: u32 = 300_000;

/// Processes a UDS request addressed to this entity.
pub trait UdsHandler {
    /// Handles `request` and writes the UDS response into `response`.
    /// Returns the response length; 0 means no response (e.g. suppressed).
    fn handle(&mut self, source: u16, target: u16, request: &[u8], response: &mut [u8]) -> usize;

    /// True if handling `request` takes long enough (e.g. a flash erase) that
    /// a `7F <SID> 78` responsePending must be sent first.
    fn response_pending(&self, _request: &[u8]) -> bool {
        false
    }
}

/// The transport could not send a frame (peer gone, out of buffers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendError;

/// Sends a complete DoIP frame to the peer.
pub trait Transport {
    fn send(&mut self, frame: &[u8]) -> Result<(), SendError>;
}

/// Reasons a connection must be closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The transport failed to send.
    Send,
    /// Header pattern check failed (generic NACK sent).
    IncorrectPattern,
    /// Payload larger than [`MAX_PAYLOAD_SIZE`] (generic NACK sent).
    MessageTooLarge,
    /// A known payload type had an invalid length.
    InvalidPayloadLength,
}

/// State of a single TCP connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    AwaitingActivation,
    Active { tester_address: u16 },
}

/// One DoIP TCP connection with its receive (reassembly) and transmit buffers.
pub struct Connection {
    ecu_address: u16,
    functional_address: u16,
    state: ConnectionState,
    rx: heapless::Vec<u8, FRAME_CAPACITY>,
    tx: [u8; FRAME_CAPACITY],
}

impl Connection {
    pub const fn new(ecu_address: u16, functional_address: u16) -> Self {
        Self {
            ecu_address,
            functional_address,
            state: ConnectionState::AwaitingActivation,
            rx: heapless::Vec::new(),
            tx: [0; FRAME_CAPACITY],
        }
    }

    /// Resets the connection for a new peer.
    pub fn reset(&mut self) {
        self.state = ConnectionState::AwaitingActivation;
        self.rx.clear();
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Feeds bytes received from the socket. Complete frames are processed;
    /// partial frames are kept until the rest arrives.
    pub fn on_data<H: UdsHandler, T: Transport>(
        &mut self,
        mut data: &[u8],
        uds: &mut H,
        transport: &mut T,
    ) -> Result<(), Error> {
        while !data.is_empty() {
            // Fill up to the end of the current frame (or its header).
            let wanted = self.bytes_wanted();
            let take = wanted.min(data.len());
            // Cannot fail: bytes_wanted() never exceeds the remaining capacity.
            let _ = self.rx.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.rx.len() == DoipHeader::SIZE {
                self.check_header(transport)?;
            }
            if let Some(total) = self.frame_len() {
                if self.rx.len() == total {
                    let result = self.process_frame(uds, transport);
                    self.rx.clear();
                    result?;
                }
            }
        }
        Ok(())
    }

    fn frame_len(&self) -> Option<usize> {
        DoipHeader::parse(&self.rx).map(|h| DoipHeader::SIZE + h.payload_length as usize)
    }

    fn bytes_wanted(&self) -> usize {
        match self.frame_len() {
            Some(total) => total - self.rx.len(),
            None => DoipHeader::SIZE - self.rx.len(),
        }
    }

    fn check_header<T: Transport>(&mut self, transport: &mut T) -> Result<(), Error> {
        let header = DoipHeader::parse(&self.rx).expect("header complete");
        if !header.is_valid() {
            self.send_generic_nack(generic_nack_code::INCORRECT_PATTERN, transport)?;
            return Err(Error::IncorrectPattern);
        }
        if header.payload_length as usize > MAX_PAYLOAD_SIZE {
            self.send_generic_nack(generic_nack_code::MESSAGE_TOO_LARGE, transport)?;
            return Err(Error::MessageTooLarge);
        }
        Ok(())
    }

    fn process_frame<H: UdsHandler, T: Transport>(
        &mut self,
        uds: &mut H,
        transport: &mut T,
    ) -> Result<(), Error> {
        let header = DoipHeader::parse(&self.rx).expect("header complete");
        // Copy the payload bounds out so `self` can be borrowed mutably below.
        let payload_range = DoipHeader::SIZE..self.rx.len();

        match PayloadType::from_u16(header.payload_type) {
            Some(PayloadType::RoutingActivationRequest) => {
                self.handle_routing_activation(payload_range, transport)
            }
            Some(PayloadType::DiagnosticMessage) => {
                self.handle_diagnostic_message(payload_range, uds, transport)
            }
            Some(PayloadType::AliveCheckRequest) => {
                let resp = AliveCheckResponse {
                    source_address: self.ecu_address,
                };
                self.send(|out| resp.encode(out), transport)
            }
            Some(_) => Ok(()), // valid type, but not expected from a tester on TCP
            None => self.send_generic_nack(generic_nack_code::UNKNOWN_PAYLOAD_TYPE, transport),
        }
    }

    fn handle_routing_activation<T: Transport>(
        &mut self,
        payload: core::ops::Range<usize>,
        transport: &mut T,
    ) -> Result<(), Error> {
        let Some(req) = RoutingActivationRequest::parse(&self.rx[payload]) else {
            self.send_generic_nack(generic_nack_code::INVALID_PAYLOAD_LENGTH, transport)?;
            return Err(Error::InvalidPayloadLength);
        };

        let resp = RoutingActivationResponse {
            tester_address: req.source_address,
            entity_address: self.ecu_address,
            response_code: routing_response_code::SUCCESS,
            reserved: 0,
        };
        self.send(|out| resp.encode(out), transport)?;
        self.state = ConnectionState::Active {
            tester_address: req.source_address,
        };
        Ok(())
    }

    fn handle_diagnostic_message<H: UdsHandler, T: Transport>(
        &mut self,
        payload: core::ops::Range<usize>,
        uds: &mut H,
        transport: &mut T,
    ) -> Result<(), Error> {
        let ConnectionState::Active { tester_address } = self.state else {
            // Diagnostic message before routing activation: ignored.
            return Ok(());
        };

        let Some(msg) = DiagnosticMessage::parse(&self.rx[payload]) else {
            self.send_generic_nack(generic_nack_code::INVALID_PAYLOAD_LENGTH, transport)?;
            return Err(Error::InvalidPayloadLength);
        };
        let (source, target) = (msg.source_address, msg.target_address);

        let nack_code = if source != tester_address {
            Some(diag_nack_code::INVALID_SOURCE_ADDRESS)
        } else if target != self.ecu_address && target != self.functional_address {
            Some(diag_nack_code::UNKNOWN_TARGET_ADDRESS)
        } else {
            None
        };
        if let Some(nack_code) = nack_code {
            let nack = DiagnosticMessageNack {
                source_address: self.ecu_address,
                target_address: source,
                nack_code,
            };
            return self.send(|out| nack.encode(out), transport);
        }

        let ack = DiagnosticMessageAck {
            source_address: self.ecu_address,
            target_address: source,
            ack_code: 0x00,
        };
        self.send(|out| ack.encode(out), transport)?;

        let request = &self.rx[DoipHeader::SIZE + 4..];
        if !request.is_empty() && uds.response_pending(request) {
            let pending = [0x7F, request[0], 0x78];
            let msg = DiagnosticMessage {
                source_address: self.ecu_address,
                target_address: source,
                user_data: &pending,
            };
            let len = msg.encode(&mut self.tx).expect("fits");
            transport
                .send(&self.tx[..len])
                .map_err(|SendError| Error::Send)?;
        }

        // The UDS response is written straight into the transmit buffer behind
        // the space reserved for the DoIP header and addresses.
        let request = &self.rx[DoipHeader::SIZE + 4..];
        let response_len = uds.handle(source, target, request, &mut self.tx[DIAG_DATA_OFFSET..]);
        if response_len == 0 {
            return Ok(());
        }

        let frame_len = DIAG_DATA_OFFSET + response_len;
        let header = DoipHeader::new(PayloadType::DiagnosticMessage, (4 + response_len) as u32);
        self.tx[..DoipHeader::SIZE].copy_from_slice(&header.encode());
        self.tx[DoipHeader::SIZE..DoipHeader::SIZE + 2]
            .copy_from_slice(&self.ecu_address.to_be_bytes());
        self.tx[DoipHeader::SIZE + 2..DIAG_DATA_OFFSET].copy_from_slice(&source.to_be_bytes());
        transport
            .send(&self.tx[..frame_len])
            .map_err(|SendError| Error::Send)
    }

    fn send_generic_nack<T: Transport>(
        &mut self,
        code: u8,
        transport: &mut T,
    ) -> Result<(), Error> {
        let nack = GenericNack { nack_code: code };
        self.send(|out| nack.encode(out), transport)
    }

    fn send<T: Transport>(
        &mut self,
        encode: impl FnOnce(&mut [u8]) -> Option<usize>,
        transport: &mut T,
    ) -> Result<(), Error> {
        let len = encode(&mut self.tx).expect("control frames fit the transmit buffer");
        transport
            .send(&self.tx[..len])
            .map_err(|SendError| Error::Send)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    struct Echo;
    impl UdsHandler for Echo {
        fn handle(&mut self, _s: u16, _t: u16, req: &[u8], resp: &mut [u8]) -> usize {
            resp[0] = req[0] + 0x40;
            resp[1..req.len()].copy_from_slice(&req[1..]);
            req.len()
        }
    }

    #[derive(Default)]
    struct Sink(Vec<Vec<u8>>);
    impl Transport for Sink {
        fn send(&mut self, frame: &[u8]) -> Result<(), SendError> {
            self.0.push(frame.to_vec());
            Ok(())
        }
    }

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.split_whitespace().collect();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    const RA: &str = "02FD 0005 00000007 0E00 00 00000000";
    const TESTER_PRESENT: &str = "02FD 8001 00000006 0E00 1000 3E00";

    fn conn() -> Box<Connection> {
        Box::new(Connection::new(0x1000, 0xFFFF))
    }

    #[test]
    fn routing_activation_then_tester_present() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex(RA), &mut Echo, &mut sink).unwrap();
        c.on_data(&hex(TESTER_PRESENT), &mut Echo, &mut sink)
            .unwrap();

        assert_eq!(sink.0[0], hex("02FD 0006 00000009 0E00 1000 10 00000000"));
        assert_eq!(sink.0[1], hex("02FD 8002 00000005 1000 0E00 00"));
        assert_eq!(sink.0[2], hex("02FD 8001 00000006 1000 0E00 7E00"));
    }

    #[test]
    fn frames_split_across_reads_and_coalesced() {
        let (mut c, mut sink) = (conn(), Sink::default());
        let mut stream = hex(RA);
        stream.extend(hex(TESTER_PRESENT));
        for chunk in stream.chunks(3) {
            c.on_data(chunk, &mut Echo, &mut sink).unwrap();
        }
        assert_eq!(sink.0.len(), 3);
        assert_eq!(sink.0[2], hex("02FD 8001 00000006 1000 0E00 7E00"));
    }

    #[test]
    fn diagnostic_message_before_activation_is_ignored() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex(TESTER_PRESENT), &mut Echo, &mut sink)
            .unwrap();
        assert!(sink.0.is_empty());
    }

    #[test]
    fn unknown_target_is_nacked() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex(RA), &mut Echo, &mut sink).unwrap();
        c.on_data(
            &hex("02FD 8001 00000006 0E00 2000 3E00"),
            &mut Echo,
            &mut sink,
        )
        .unwrap();
        assert_eq!(sink.0[1], hex("02FD 8003 00000005 1000 0E00 03"));
    }

    #[test]
    fn wrong_source_is_nacked() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex(RA), &mut Echo, &mut sink).unwrap();
        c.on_data(
            &hex("02FD 8001 00000006 0E01 1000 3E00"),
            &mut Echo,
            &mut sink,
        )
        .unwrap();
        assert_eq!(sink.0[1], hex("02FD 8003 00000005 1000 0E01 02"));
    }

    #[test]
    fn bad_pattern_closes_with_generic_nack() {
        let (mut c, mut sink) = (conn(), Sink::default());
        let err = c.on_data(&hex("02FC 0005 00000007"), &mut Echo, &mut sink);
        assert_eq!(err, Err(Error::IncorrectPattern));
        assert_eq!(sink.0[0], hex("02FD 0000 00000001 00"));
    }

    #[test]
    fn oversized_payload_closes_with_generic_nack() {
        let (mut c, mut sink) = (conn(), Sink::default());
        let err = c.on_data(&hex("02FD 8001 00010000"), &mut Echo, &mut sink);
        assert_eq!(err, Err(Error::MessageTooLarge));
        assert_eq!(sink.0[0], hex("02FD 0000 00000001 02"));
    }

    #[test]
    fn unknown_payload_type_is_nacked_and_skipped() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex("02FD 4444 00000002 AABB"), &mut Echo, &mut sink)
            .unwrap();
        c.on_data(&hex(RA), &mut Echo, &mut sink).unwrap();
        assert_eq!(sink.0[0], hex("02FD 0000 00000001 01"));
        assert_eq!(
            c.state(),
            ConnectionState::Active {
                tester_address: 0x0E00
            }
        );
    }

    struct Slow;
    impl UdsHandler for Slow {
        fn handle(&mut self, _s: u16, _t: u16, _req: &[u8], resp: &mut [u8]) -> usize {
            resp[..4].copy_from_slice(&[0x74, 0x20, 0x0F, 0xFF]);
            4
        }
        fn response_pending(&self, request: &[u8]) -> bool {
            request[0] == 0x34
        }
    }

    #[test]
    fn slow_request_gets_response_pending_first() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex(RA), &mut Slow, &mut sink).unwrap();
        c.on_data(
            &hex("02FD 8001 00000007 0E00 1000 34 00 44"),
            &mut Slow,
            &mut sink,
        )
        .unwrap();
        assert_eq!(sink.0[1], hex("02FD 8002 00000005 1000 0E00 00"));
        assert_eq!(sink.0[2], hex("02FD 8001 00000007 1000 0E00 7F3478"));
        assert_eq!(sink.0[3], hex("02FD 8001 00000008 1000 0E00 74200FFF"));
    }

    #[test]
    fn alive_check() {
        let (mut c, mut sink) = (conn(), Sink::default());
        c.on_data(&hex("02FD 0007 00000000"), &mut Echo, &mut sink)
            .unwrap();
        assert_eq!(sink.0[0], hex("02FD 0008 00000002 1000"));
    }
}

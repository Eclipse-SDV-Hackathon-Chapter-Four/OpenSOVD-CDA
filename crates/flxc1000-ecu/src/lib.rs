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

//! FLXC1000 ECU behaviour, independent of the transport and the hardware.
//! The diagnostic surface is specified in `docs/diagnostics.md`.

#![cfg_attr(not(test), no_std)]

pub mod app;
pub mod board;
pub mod boot;

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use ace_server::handler::ServerHandler;
use ace_server::security_provider::SecurityProvider;
use ace_server::server::{ServerError, UdsServer, MAX_FRAME, MAX_OUTBOX};
use ace_sim::clock::Instant as AceInstant;
use ace_sim::io::NodeAddress;

pub use app::AppEcu;
pub use board::{Board, BootState, FlashError, Sensor};
pub use boot::BootEcu;

/// 0x1001: distinct from the Raspberry Pi FLXC1000 (0x1000) on the same network.
pub const ECU_ADDRESS: u16 = 0x1001;
pub const FUNCTIONAL_ADDRESS: u16 = 0xFFFF;
pub const VIN: &[u8; 17] = b"FLXC1000AZ3166001";

/// DID 0xF195, space-padded crate version
pub const SOFTWARE_VERSION: [u8; 8] = pad_version(env!("CARGO_PKG_VERSION").as_bytes());

pub const RESET_NONE: u8 = 0;
pub const RESET_HARD: u8 = 1;
pub const RESET_SOFT: u8 = 3;

/// Routine status codes (routineStatusRecord byte)
pub const ROUTINE_IDLE: u8 = 0x00;
pub const ROUTINE_RUNNING: u8 = 0x01;
pub const ROUTINE_COMPLETED: u8 = 0x02;
pub const ROUTINE_ABORTED: u8 = 0x03;

const fn pad_version(v: &[u8]) -> [u8; 8] {
    let mut out = [b' '; 8];
    let mut i = 0;
    while i < v.len() && i < 8 {
        out[i] = v[i];
        i += 1;
    }
    out
}

/// State shared between the UDS server (behind the ECU lock) and the
/// firmware tasks (routine task, reset handling).
pub struct Shared {
    pub pending_reset: AtomicU8,
    pub session_type: AtomicU8,
    pub routine_status: AtomicU8,
    /// Set by RoutineControl Start, consumed by the routine task.
    pub routine_start: AtomicBool,
}

impl Shared {
    pub const fn new() -> Self {
        Self {
            pending_reset: AtomicU8::new(RESET_NONE),
            session_type: AtomicU8::new(0x01),
            routine_status: AtomicU8::new(ROUTINE_IDLE),
            routine_start: AtomicBool::new(false),
        }
    }

    /// Returns and clears a reset requested by ECUReset.
    pub fn take_pending_reset(&self) -> u8 {
        self.pending_reset.swap(RESET_NONE, Ordering::SeqCst)
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

/// DIDs present in both variants.
pub(crate) fn common_did<B: Board>(
    did: u16,
    variant_id: &[u8; 3],
    shared: &Shared,
    board: &B,
    buf: &mut [u8],
) -> Option<usize> {
    match did {
        0xF100 => {
            buf[..3].copy_from_slice(variant_id);
            Some(3)
        }
        0xF186 => {
            buf[0] = shared.session_type.load(Ordering::Relaxed);
            Some(1)
        }
        0xF18C => {
            buf[..12].copy_from_slice(&board.serial_number());
            Some(12)
        }
        0xF195 => {
            buf[..8].copy_from_slice(&SOFTWARE_VERSION);
            Some(8)
        }
        _ => None,
    }
}

type Outbox = heapless::Vec<(NodeAddress, heapless::Vec<u8, MAX_FRAME>), MAX_OUTBOX>;

const NRC_SERVICE_NOT_SUPPORTED: u8 = 0x11;
const NRC_SUB_FUNCTION_NOT_SUPPORTED: u8 = 0x12;
const NRC_INCORRECT_LENGTH: u8 = 0x13;
const NRC_SECURITY_ACCESS_DENIED: u8 = 0x33;

/// Checks ace-server (at the pinned revision) does not do itself:
/// unknown services, ECUReset sub-functions (it answers positively before
/// asking the handler) and service-level security.
pub(crate) struct Policy {
    /// All services of the variant, including ones handled outside ace-server.
    pub supported_sids: &'static [u8],
    pub reset_types: &'static [u8],
    /// Services requiring `security_level` when in one of `secured_sessions`.
    pub secured_sids: &'static [u8],
    pub secured_sessions: &'static [u8],
    pub security_level: u8,
}

impl Policy {
    /// Returns the NRC to send instead of dispatching the request, if any.
    fn check(&self, request: &[u8], session_type: u8, security_level: u8) -> Option<u8> {
        let sid = request[0];
        if !self.supported_sids.contains(&sid) {
            return Some(NRC_SERVICE_NOT_SUPPORTED);
        }
        if sid == 0x11 {
            let reset_type = request.get(1)? & 0x7F;
            if !self.reset_types.contains(&reset_type) {
                return Some(NRC_SUB_FUNCTION_NOT_SUPPORTED);
            }
        }
        // Session checks stay with the server (NRC 7F takes precedence).
        if self.secured_sids.contains(&sid)
            && self.secured_sessions.contains(&session_type)
            && security_level < self.security_level
        {
            return Some(NRC_SECURITY_ACCESS_DENIED);
        }
        None
    }
}

fn negative_response(response: &mut [u8], sid: u8, nrc: u8) -> usize {
    response[..3].copy_from_slice(&[0x7F, sid, nrc]);
    3
}

pub enum Variant<B: Board> {
    App(AppEcu<B>),
    Boot(BootEcu<B>),
}

/// The ECU: one variant plus the scratch space to collect server responses.
/// Large (~130 KiB) — the firmware keeps it in a static.
pub struct Ecu<B: Board> {
    variant: Variant<B>,
    shared: &'static Shared,
    outbox: Outbox,
}

impl<B: Board + Clone> Ecu<B> {
    pub fn new(state: BootState, board: B, shared: &'static Shared) -> Self {
        let variant = match state {
            BootState::AppValid => Variant::App(AppEcu::new(board, shared)),
            BootState::BootRequested => Variant::Boot(BootEcu::new(board, shared)),
        };
        Self {
            variant,
            shared,
            outbox: heapless::Vec::new(),
        }
    }
}

impl<B: Board> Ecu<B> {
    pub fn is_app(&self) -> bool {
        matches!(self.variant, Variant::App(_))
    }

    /// Handles one UDS request from tester `source`; writes the response into
    /// `response` and returns its length (0 = no response).
    pub fn handle(
        &mut self,
        source: u16,
        request: &[u8],
        response: &mut [u8],
        now_us: u64,
    ) -> usize {
        if request.is_empty() {
            response[..3].copy_from_slice(&[0x7F, 0x00, 0x13]);
            return 3;
        }
        let now = AceInstant::from_micros(now_us);
        let sid = request[0];
        match &mut self.variant {
            Variant::App(app) => {
                let server = &app.server;
                if let Some(nrc) =
                    app::POLICY.check(request, server.session_type(), server.security_level())
                {
                    return negative_response(response, sid, nrc);
                }
                if let Some(n) = app.handle_local(request, response) {
                    return n;
                }
                serve(
                    &mut app.server,
                    self.shared,
                    &mut self.outbox,
                    source,
                    request,
                    response,
                    now,
                )
            }
            Variant::Boot(boot) => {
                let server = &boot.server;
                if let Some(nrc) =
                    boot::POLICY.check(request, server.session_type(), server.security_level())
                {
                    return negative_response(response, sid, nrc);
                }
                serve(
                    &mut boot.server,
                    self.shared,
                    &mut self.outbox,
                    source,
                    request,
                    response,
                    now,
                )
            }
        }
    }

    /// Advances server timers (S3 session timeout). Call periodically.
    pub fn tick(&mut self, now_us: u64) {
        let now = AceInstant::from_micros(now_us);
        match &mut self.variant {
            Variant::App(app) => tick(&mut app.server, self.shared, &mut self.outbox, now),
            Variant::Boot(boot) => tick(&mut boot.server, self.shared, &mut self.outbox, now),
        }
    }
}

fn serve<H: ServerHandler, S: SecurityProvider>(
    server: &mut UdsServer<H, S>,
    shared: &Shared,
    outbox: &mut Outbox,
    source: u16,
    request: &[u8],
    response: &mut [u8],
    now: AceInstant,
) -> usize {
    let src = NodeAddress(source as u32);
    shared
        .session_type
        .store(server.session_type(), Ordering::Relaxed);
    let result = server.handle(&src, request, now);
    let _ = server.tick(now);
    shared
        .session_type
        .store(server.session_type(), Ordering::Relaxed);

    outbox.clear();
    server.drain_outbox(outbox);

    // ace-server returns handler errors instead of queueing the NRC.
    match result {
        Err(ServerError::Handler(e)) => return negative_response(response, request[0], e.into()),
        Err(ServerError::Codec(_)) => {
            return negative_response(response, request[0], NRC_INCORRECT_LENGTH)
        }
        _ => {}
    }
    match outbox.iter().find(|(dst, _)| dst == &src) {
        Some((_, data)) => {
            let n = data.len().min(response.len());
            response[..n].copy_from_slice(&data[..n]);
            n
        }
        None => 0,
    }
}

fn tick<H: ServerHandler, S: SecurityProvider>(
    server: &mut UdsServer<H, S>,
    shared: &Shared,
    outbox: &mut Outbox,
    now: AceInstant,
) {
    let _ = server.tick(now);
    shared
        .session_type
        .store(server.session_type(), Ordering::Relaxed);
    // Periodic DIDs are not configured; discard anything queued.
    outbox.clear();
    server.drain_outbox(outbox);
    outbox.clear();
}

#[cfg(test)]
mod tests;

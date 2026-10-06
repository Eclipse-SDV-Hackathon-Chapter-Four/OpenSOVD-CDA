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

//! Boot variant: programming session, security access, download.

use core::sync::atomic::Ordering;

use ace_server::config::*;
use ace_server::handler::ServerHandler;
use ace_server::nrc::BuiltinNrc;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::UdsServer;
use ace_sim::clock::Duration as AceDuration;
use ace_sim::io::NodeAddress;

use crate::board::{Board, FlashError};
use crate::update::{self, Block, Download, UpdateError};
use crate::{
    common_did, Policy, Shared, ECU_ADDRESS, FUNCTIONAL_ADDRESS, OUTBOX, RESET_HARD, RESET_SOFT,
};

/// Boot variant identification: DID 0xF100 = 0xFF0000
pub const VARIANT_ID: [u8; 3] = [0xFF, 0x00, 0x00];

/// XOR mask of the seed/key algorithm.
pub const KEY_MASK: u32 = 0xDEAD_BEEF;

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

pub struct BootHandler<B: Board> {
    board: B,
    shared: &'static Shared,
    download: Option<Download>,
}

/// Parses a big-endian address / size field of RequestDownload.
fn be_value(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 4 {
        return None;
    }
    Some(bytes.iter().fold(0u32, |acc, b| (acc << 8) | *b as u32))
}

impl<B: Board> ServerHandler for BootHandler<B> {
    type Error = BuiltinNrc;

    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, BuiltinNrc> {
        common_did(did, &VARIANT_ID, self.shared, &self.board, buf)
            .ok_or(BuiltinNrc::RequestOutOfRange)
    }

    fn write_did(&mut self, _did: u16, _data: &[u8]) -> Result<(), BuiltinNrc> {
        Err(BuiltinNrc::RequestOutOfRange)
    }

    fn ecu_reset(&mut self, reset_type: u8) -> Result<(), BuiltinNrc> {
        let reset = match reset_type {
            0x01 => RESET_HARD,
            0x03 => RESET_SOFT,
            _ => return Err(BuiltinNrc::SubFunctionNotSupported),
        };
        self.shared.pending_reset.store(reset, Ordering::SeqCst);
        Ok(())
    }

    fn request_download(
        &mut self,
        memory_address: &[u8],
        memory_size: &[u8],
        _compression_method: u8,
        _encrypting_method: u8,
        buf: &mut [u8],
    ) -> Result<usize, BuiltinNrc> {
        let (Some(address), Some(size)) = (be_value(memory_address), be_value(memory_size)) else {
            return Err(BuiltinNrc::RequestOutOfRange);
        };
        // The package goes to the app area; the slot is picked here.
        if address != update::APP_REGION
            || size < update::HEADER_SIZE as u32
            || size > update::MAX_PACKAGE_SIZE
        {
            return Err(BuiltinNrc::RequestOutOfRange);
        }
        // A repeated RequestDownload (tester retry) restarts the download.
        self.download = None;
        let target = self
            .board
            .update_begin()
            .ok_or(BuiltinNrc::UploadDownloadNotAccepted)?;
        self.download = Some(Download::new(target, size));
        // lengthFormatIdentifier: 0x20 (2 bytes for maxBlockLength)
        // maxNumberOfBlockLength: 0x0FFF (4095 bytes: SID + counter + 4093 data)
        buf[..3].copy_from_slice(&[0x20, 0x0F, 0xFF]);
        Ok(3)
    }

    fn transfer_data(
        &mut self,
        block_sequence_counter: u8,
        data: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, BuiltinNrc> {
        let download = self
            .download
            .as_mut()
            .ok_or(BuiltinNrc::RequestSequenceError)?;
        match download.check_block(block_sequence_counter) {
            // ace-server echoes the block sequence counter itself.
            Block::Repeat => Ok(0),
            Block::Wrong => Err(BuiltinNrc::WrongBlockSequenceCounter),
            Block::Next => match download.write_block(block_sequence_counter, data, &self.board) {
                Ok(()) => Ok(0),
                Err(UpdateError::TooMuchData) => Err(BuiltinNrc::TransferDataSuspended),
                Err(UpdateError::BadPackage | UpdateError::NoImageForSlot) => {
                    Err(BuiltinNrc::RequestOutOfRange)
                }
                Err(_) => Err(BuiltinNrc::GeneralProgrammingFailure),
            },
        }
    }

    fn request_transfer_exit(
        &mut self,
        _parameter_record: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, BuiltinNrc> {
        let download = self
            .download
            .take()
            .ok_or(BuiltinNrc::RequestSequenceError)?;
        download
            .verify()
            .map_err(|_| BuiltinNrc::GeneralProgrammingFailure)?;
        // Download complete: the next hard reset starts the new app on trial.
        self.board
            .update_commit(download.target())
            .map_err(|FlashError| BuiltinNrc::GeneralProgrammingFailure)?;
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// SecurityProvider — seed/key with XOR 0xDEADBEEF
// ---------------------------------------------------------------------------

pub struct BootSecurity<B: Board> {
    board: B,
}

impl<B: Board> SecurityProvider for BootSecurity<B> {
    fn generate_seed(&mut self, _level: u8, buf: &mut [u8]) -> Result<usize, SecurityError> {
        // A zero seed means "already unlocked" in UDS — avoid it.
        let seed = self.board.random_u32().max(1);
        buf[..4].copy_from_slice(&seed.to_be_bytes());
        Ok(4)
    }

    fn validate_key(&self, _level: u8, seed: &[u8], key: &[u8]) -> Result<(), SecurityError> {
        if seed.len() < 4 || key.len() < 4 {
            return Err(SecurityError::InvalidKey);
        }
        let seed_u32 = u32::from_be_bytes([seed[0], seed[1], seed[2], seed[3]]);
        let actual = u32::from_be_bytes([key[0], key[1], key[2], key[3]]);
        if actual == seed_u32 ^ KEY_MASK {
            Ok(())
        } else {
            Err(SecurityError::InvalidKey)
        }
    }
}

// ---------------------------------------------------------------------------
// Server config
// ---------------------------------------------------------------------------

const ALL: &[u8] = &[0x01, 0x02, 0x03];
const PROG_EXT: &[u8] = &[0x02, 0x03];
const PROG: &[u8] = &[0x02];

const SECURITY_LEVEL: u8 = 0x03;

pub(crate) const POLICY: Policy = Policy {
    supported_sids: &[0x10, 0x11, 0x22, 0x27, 0x34, 0x36, 0x37, 0x3E],
    reset_types: &[0x01, 0x03],
    secured_sids: &[0x34, 0x36, 0x37],
    secured_sessions: PROG,
    security_level: SECURITY_LEVEL,
};

pub fn boot_server_config() -> ServerConfig {
    ServerConfig::new(ECU_ADDRESS, FUNCTIONAL_ADDRESS)
        // Sessions
        .with_session(SessionConfig::default_session())
        .with_session(SessionConfig::programming_session())
        .with_session(SessionConfig::extended_session())
        // Services: (SID, allowed sessions)
        .with_service(ServiceConfig::new(0x10, ALL)) // DiagnosticSessionControl
        .with_service(ServiceConfig::new(0x11, ALL)) // ECUReset
        .with_service(ServiceConfig::new(0x22, ALL)) // ReadDataByIdentifier
        .with_service(ServiceConfig::new(0x27, PROG_EXT)) // SecurityAccess
        .with_service(ServiceConfig::secured(0x34, PROG, SECURITY_LEVEL)) // RequestDownload
        .with_service(ServiceConfig::secured(0x36, PROG, SECURITY_LEVEL)) // TransferData
        .with_service(ServiceConfig::secured(0x37, PROG, SECURITY_LEVEL)) // RequestTransferExit
        .with_service(ServiceConfig::new(0x3E, ALL)) // TesterPresent
        // DIDs
        .with_did(DidConfig::read_only(0xF100, ALL)) // variant ID
        .with_did(DidConfig::read_only(0xF186, ALL)) // session
        .with_did(DidConfig::read_only(0xF18C, ALL)) // serial number
        .with_did(DidConfig::read_only(0xF195, ALL)) // software version
        // Security levels
        .with_security_level(SecurityLevelConfig {
            level: SECURITY_LEVEL,
            max_attempts: 3,
            lockout_duration: AceDuration::from_secs(10),
            seed_length: 4,
            key_length: 4,
        })
}

pub struct BootEcu<B: Board> {
    pub(crate) server: UdsServer<BootHandler<B>, BootSecurity<B>, OUTBOX>,
}

impl<B: Board + Clone> BootEcu<B> {
    pub fn new(board: B, shared: &'static Shared) -> Self {
        let handler = BootHandler {
            board: board.clone(),
            shared,
            download: None,
        };
        Self {
            server: UdsServer::new(
                boot_server_config(),
                handler,
                BootSecurity { board },
                NodeAddress(ECU_ADDRESS as u32),
            ),
        }
    }
}

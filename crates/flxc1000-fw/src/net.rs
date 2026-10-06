/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! DoIP over NetX Duo: one thread per TCP slot plus the UDP thread.

use core::cell::UnsafeCell;

use flxc1000_doip::connection::INACTIVITY_TIMEOUT_MS;
use flxc1000_doip::message::VehicleAnnouncement;
use flxc1000_doip::{udp, Connection, DoipConfig, SendError, Transport, UdsHandler};
use flxc1000_ecu::{ECU_ADDRESS, FUNCTIONAL_ADDRESS, VIN};

use crate::sys;

const DOIP_PORT: u16 = 13400;
const BROADCAST: u32 = 0xFFFF_FFFF;

/// ISO 13400-2: A_DoIP_Announce_Num / A_DoIP_Announce_Interval
const ANNOUNCE_COUNT: u32 = 3;
const ANNOUNCE_INTERVAL_MS: u32 = 500;

const RECV_TIMEOUT_MS: u32 = 1000;
/// One TCP segment; the connection reassembles frames.
const RECV_CHUNK: usize = 1460;

/// Per-slot connection state (~8 KiB each), kept out of the thread stacks.
struct SlotCell(UnsafeCell<Connection>);

// SAFETY: slot `n` is only used by the TCP thread for slot `n`.
unsafe impl Sync for SlotCell {}

static SLOTS: [SlotCell; sys::TCP_SLOTS] = [
    SlotCell(UnsafeCell::new(Connection::new(
        ECU_ADDRESS,
        FUNCTIONAL_ADDRESS,
    ))),
    SlotCell(UnsafeCell::new(Connection::new(
        ECU_ADDRESS,
        FUNCTIONAL_ADDRESS,
    ))),
];

struct TcpTransport(u32);

impl Transport for TcpTransport {
    fn send(&mut self, frame: &[u8]) -> Result<(), SendError> {
        match unsafe { sys::plat_tcp_send(self.0, frame.as_ptr(), frame.len() as u32) } {
            0 => Ok(()),
            _ => Err(SendError),
        }
    }
}

/// Hands UDS requests to the UDS worker thread.
struct Worker;

impl UdsHandler for Worker {
    fn handle(&mut self, source: u16, _target: u16, request: &[u8], response: &mut [u8]) -> usize {
        unsafe {
            sys::plat_uds_execute(
                source,
                request.as_ptr(),
                request.len() as u32,
                response.as_mut_ptr(),
                response.len() as u32,
            ) as usize
        }
    }
}

/// DoIP TCP slot thread.
#[no_mangle]
pub extern "C" fn flxc1000_tcp_task(slot: u32) -> ! {
    // SAFETY: this thread is the only user of its slot.
    let connection = unsafe { &mut *SLOTS[slot as usize].0.get() };
    let mut buf = [0u8; RECV_CHUNK];

    loop {
        if unsafe { sys::plat_tcp_accept(slot) } != 0 {
            sys::sleep_ms(1000);
            continue;
        }
        log!("DoIP slot {}: tester connected", slot);
        connection.reset();
        serve(slot, connection, &mut buf);
        unsafe { sys::plat_tcp_close(slot) };
        log!("DoIP slot {}: closed", slot);
    }
}

fn serve(slot: u32, connection: &mut Connection, buf: &mut [u8]) {
    let mut idle_ms = 0;
    loop {
        let n = unsafe {
            sys::plat_tcp_recv(slot, buf.as_mut_ptr(), buf.len() as u32, RECV_TIMEOUT_MS)
        };
        if n == sys::TIMEOUT {
            idle_ms += RECV_TIMEOUT_MS;
            if idle_ms >= INACTIVITY_TIMEOUT_MS {
                log!("DoIP slot {}: inactivity timeout", slot);
                return;
            }
            continue;
        }
        if n < 0 {
            return;
        }
        idle_ms = 0;
        let result = connection.on_data(&buf[..n as usize], &mut Worker, &mut TcpTransport(slot));
        crate::handle_pending_reset();
        if let Err(e) = result {
            log!("DoIP slot {}: {:?}", slot, e);
            return;
        }
    }
}

/// DoIP UDP thread: vehicle announcements and identification responses.
#[no_mangle]
pub extern "C" fn flxc1000_udp_task() -> ! {
    // Wait for DHCP: the announcement must carry a usable source address.
    while sys::net_ip() == [0; 4] {
        sys::sleep_ms(200);
    }
    let config = DoipConfig::new(ECU_ADDRESS, VIN).with_eid(sys::net_mac());

    let mut out = [0u8; VehicleAnnouncement::FRAME_SIZE];
    if let Some(n) = config.announcement(&mut out) {
        for _ in 0..ANNOUNCE_COUNT {
            send_udp(BROADCAST, DOIP_PORT, &out[..n]);
            sys::sleep_ms(ANNOUNCE_INTERVAL_MS);
        }
    }

    let mut buf = [0u8; 256];
    loop {
        let (mut ip, mut port) = (0u32, 0u16);
        let n = unsafe {
            sys::plat_udp_recv(
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut ip,
                &mut port,
                RECV_TIMEOUT_MS,
            )
        };
        if n <= 0 {
            continue;
        }
        if let Some(len) = udp::handle_datagram(&buf[..n as usize], &config, &mut out) {
            send_udp(ip, port, &out[..len]);
        }
    }
}

fn send_udp(ip: u32, port: u16, frame: &[u8]) {
    unsafe { sys::plat_udp_send(ip, port, frame.as_ptr(), frame.len() as u32) };
}

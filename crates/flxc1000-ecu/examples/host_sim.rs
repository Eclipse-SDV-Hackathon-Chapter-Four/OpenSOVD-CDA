/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Runs the FLXC1000 ECU and DoIP stack on the host (no board needed).
//! Sensors return fixed values; LEDs, RGB and the display are logged.
//!
//!   cargo run -p flxc1000-ecu --example host_sim [bind-ip]
//!
//! Default bind address 127.0.0.1, port 13400. ECUReset switches variants
//! like on the board.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use flxc1000_doip::{udp, Connection, DoipConfig, SendError, Transport, UdsHandler};
use flxc1000_ecu::board::{Board, BootState, FlashError, Sensor};
use flxc1000_ecu::{app, Ecu, Shared, ECU_ADDRESS, FUNCTIONAL_ADDRESS, RESET_NONE, VIN};

const PORT: u16 = 13400;
const MAC: [u8; 6] = [0x02, 0x00, 0xF1, 0xC0, 0x10, 0x00];

#[derive(Clone)]
struct SimBoard {
    start: Instant,
    boot_state: Arc<Mutex<BootState>>,
    ip: [u8; 4],
}

impl Board for SimBoard {
    fn set_led(&self, index: usize, duty: u8) {
        println!("[board] LED{} = {}%", index + 1, duty);
    }
    fn sleep_ms(&self, ms: u32) {
        std::thread::sleep(Duration::from_millis(ms as u64));
    }
    fn uptime_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn random_u32(&self) -> u32 {
        self.start
            .elapsed()
            .subsec_nanos()
            .wrapping_mul(2_654_435_761)
    }
    fn serial_number(&self) -> [u8; 12] {
        *b"HOSTSIM00001"
    }
    fn ip_address(&self) -> [u8; 4] {
        self.ip
    }
    fn mac_address(&self) -> [u8; 6] {
        MAC
    }
    fn sensor_ok(&self, _sensor: Sensor) -> bool {
        true
    }
    fn temperature_humidity(&self) -> Option<(f32, f32)> {
        Some((23.1, 41.5))
    }
    fn pressure_hpa(&self) -> Option<f32> {
        Some(1013.2)
    }
    fn acceleration_mg(&self) -> Option<[f32; 3]> {
        Some([-20.0, 15.0, 1002.0])
    }
    fn angular_rate_mdps(&self) -> Option<[f32; 3]> {
        Some([350.0, -700.0, 70.0])
    }
    fn magnetic_field_mg(&self) -> Option<[f32; 3]> {
        Some([210.0, -45.0, -380.0])
    }
    fn buttons(&self) -> u8 {
        0
    }
    fn set_rgb(&self, rgb: [u8; 3]) {
        println!("[board] RGB = {:?}", rgb);
    }
    fn set_display_text(&self, text: &[u8; 16]) {
        println!("[board] display: {}", String::from_utf8_lossy(text));
    }
    fn write_boot_state(&self, state: BootState) -> Result<(), FlashError> {
        println!("[board] boot state -> {:?}", state);
        *self.boot_state.lock().unwrap() = state;
        Ok(())
    }
}

struct Sim {
    ecu: Box<Ecu<SimBoard>>,
    board: SimBoard,
    shared: &'static Shared,
}

impl Sim {
    fn new(board: SimBoard) -> Self {
        let shared: &'static Shared = Box::leak(Box::new(Shared::new()));
        let state = *board.boot_state.lock().unwrap();
        println!("[sim] starting {:?} variant", state);
        Self {
            ecu: Box::new(Ecu::new(state, board.clone(), shared)),
            board,
            shared,
        }
    }

    /// Simulated reset: rebuild the ECU from the stored boot state.
    fn reset(&mut self) {
        *self = Sim::new(self.board.clone());
    }
}

struct Handler<'a>(&'a Mutex<Sim>);

impl UdsHandler for Handler<'_> {
    fn handle(&mut self, source: u16, _target: u16, request: &[u8], response: &mut [u8]) -> usize {
        let mut sim = self.0.lock().unwrap();
        let now = sim.board.start.elapsed().as_micros() as u64;
        println!("[uds] -> {:02X?}", request);
        let n = sim.ecu.handle(source, request, response, now);
        println!("[uds] <- {:02X?}", &response[..n]);
        if sim
            .shared
            .routine_start
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            let (shared, board) = (sim.shared, sim.board.clone());
            std::thread::spawn(move || app::run_self_test(shared, &board));
        }
        n
    }
}

struct Tcp<'a>(&'a mut TcpStream);

impl Transport for Tcp<'_> {
    fn send(&mut self, frame: &[u8]) -> Result<(), SendError> {
        self.0.write_all(frame).map_err(|_| SendError)
    }
}

fn serve(mut stream: TcpStream, sim: &Mutex<Sim>) {
    let mut connection = Box::new(Connection::new(ECU_ADDRESS, FUNCTIONAL_ADDRESS));
    let mut buf = [0u8; 1460];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let result = connection.on_data(&buf[..n], &mut Handler(sim), &mut Tcp(&mut stream));
        let reset = sim.lock().unwrap().shared.take_pending_reset();
        if reset != RESET_NONE {
            std::thread::sleep(Duration::from_millis(50));
            sim.lock().unwrap().reset();
            return; // like the board: the connection drops on reset
        }
        if let Err(e) = result {
            println!("[doip] closing: {:?}", e);
            return;
        }
    }
}

fn main() {
    let ip: std::net::Ipv4Addr = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("bind IPv4 address"))
        .unwrap_or(std::net::Ipv4Addr::LOCALHOST);
    let board = SimBoard {
        start: Instant::now(),
        boot_state: Arc::new(Mutex::new(BootState::AppValid)),
        ip: ip.octets(),
    };
    let sim: &'static Mutex<Sim> = Box::leak(Box::new(Mutex::new(Sim::new(board))));

    let config = DoipConfig::new(ECU_ADDRESS, VIN).with_eid(MAC);
    let udp_socket = UdpSocket::bind((ip, PORT)).expect("bind UDP");
    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        let mut out = [0u8; 64];
        loop {
            let Ok((n, peer)) = udp_socket.recv_from(&mut buf) else {
                continue;
            };
            if let Some(len) = udp::handle_datagram(&buf[..n], &config, &mut out) {
                let _ = udp_socket.send_to(&out[..len], peer);
            }
        }
    });

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        let mut s = sim.lock().unwrap();
        let now = s.board.start.elapsed().as_micros() as u64;
        s.ecu.tick(now);
    });

    let listener = TcpListener::bind((ip, PORT)).expect("bind TCP");
    println!("[sim] DoIP on {}:{} (UDP+TCP)", ip, PORT);
    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || serve(stream, sim));
    }
}

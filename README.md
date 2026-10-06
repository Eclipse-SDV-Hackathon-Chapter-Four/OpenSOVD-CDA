<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# FLXC1000-AZ3166 — ECU Simulator on the MXCHIP AZ3166

UDS-over-DoIP ECU simulator running bare-metal on the
[MXCHIP AZ3166 IoT DevKit](https://github.com/MXCHIP/IoTDevKit)
(STM32F412RG Cortex-M4F, 1 MiB flash, 256 KiB RAM, Wi-Fi) with
[Eclipse ThreadX](https://github.com/eclipse-threadx/threadx) and NetX Duo.
The ECU logic is Rust (`no_std`) on top of
[ace-server](https://github.com/samp-reston/ace).

Independent fork of [flxc1000-rpi](https://github.com/theswiftfox/flxc1000-rpi)
(Raspberry Pi 4 / Buildroot Linux). Same ECU identity, services and Boot/App
variant concept, but on a microcontroller, and the diagnostic data comes from
the board's real sensors and actuators.

The diagnostic surface is specified in [`docs/diagnostics.md`](docs/diagnostics.md);
the matching ODX/PDX/MDD for the Classic Diagnostic Adapter is in [`odx/`](odx/).

## Architecture

```
┌─────────────────────── Rust (crates/) ───────────────────────┐
│ flxc1000-fw    staticlib: ThreadX entry points, Board impl   │
│ flxc1000-ecu   App/Boot variants, DIDs, DTCs, routine        │  host-tested
│ flxc1000-doip  ISO 13400 DoIP entity, sans-IO                │  host-tested
│ ace-server     UDS server (no_std)                           │
├────────────────────── C (platform/) ─────────────────────────┤
│ main.c   threads, UDS worker queue, variant selection        │
│ net.c    WICED Wi-Fi, NetX Duo DHCP, DoIP UDP/TCP sockets    │
│ board.c  clocks, LEDs, RGB PWM, buttons, sensors, OLED, RNG  │
│ boot_state.c  persistent App/Boot selection (flash log)      │
├──────────────────── third_party/ (fetched) ──────────────────┤
│ ThreadX · NetX Duo · STM32CubeF4 HAL · AZ3166 BSP · WICED    │
└──────────────────────────────────────────────────────────────┘
```

| Thread | Prio | Stack | Role |
|--------|------|-------|------|
| `uds` | 4 | 20 KiB | builds the ECU, brings up Wi-Fi, then executes every UDS request (only thread touching the ECU) |
| `tcp0`, `tcp1` | 6 | 3 KiB | DoIP TCP connections (2 concurrent testers) |
| `udp` | 6 | 2 KiB | vehicle announcement / identification |
| `routine` | 8 | 2 KiB | SelfTest LED cascade, session on the display |

One image contains both variants. The persistent boot state in flash sector 11
picks the variant at reset (see the spec):

```
App  --11 01-->  Boot  --34/36/37 + 11 01-->  App
      (or hold button B during reset to force Boot)
```

### Board mapping

| Board | Diagnostic use |
|-------|----------------|
| HTS221, LPS22HB, LSM6DSL, LIS2MDL | DIDs `F201`–`F206`; init failures → DTCs `C1xx00` |
| Buttons A/B | DID `F210`; B held at reset → Boot variant |
| User LED, Azure LED, RGB R/G/B | SelfTest LED bar (routine `1001`); RGB via DID `F211` |
| Wi-Fi LED | on = DHCP address acquired |
| OLED | variant, IP, session, DID `F212` text |
| STM32 unique ID / RNG | DID `F18C` / SecurityAccess seed |

## Build

Prerequisites: Rust (rustup), CMake ≥ 3.20, Ninja, git, Python 3 (smoke test).
The Rust target (`thumbv7em-none-eabihf`) is installed automatically through
`rust-toolchain.toml`.

```bash
scripts/fetch-deps.sh        # ThreadX, NetX Duo, AZ3166 BSP -> third_party/
scripts/setup-toolchain.sh   # Arm GNU Toolchain 14.3 -> .toolchain/ (or use your own on PATH)

WIFI_SSID=MyNetwork WIFI_PASSWORD=secret scripts/build.sh
```

Output: `build/flxc1000.elf`, `build/flxc1000.bin`. Credentials are compiled in
(WPA2-PSK; empty password = open network) and stay in `build/CMakeCache.txt`.

`fetch-deps.sh` pins ThreadX and NetX Duo to the exact commits the prebuilt
WICED Wi-Fi library was built against: the struct layouts must match.

## Flash and run

Connect the board via USB; it shows up as a drive named `AZ3166`.

```bash
scripts/flash.sh             # copies build/flxc1000.bin to the drive
```

Console: the board's USB serial port, 115200 8N1. The display shows the
assigned IP address once Wi-Fi is up; the ECU sends three vehicle
announcements and answers on UDP/TCP 13400.

```bash
scripts/doip-smoke.py <board-ip>            # App variant checks
scripts/doip-smoke.py <board-ip> --to-boot  # plus App -> Boot -> App cycle
```

## Use with the Classic Diagnostic Adapter

The ECU is `AZ3166` at logical address `0x1001` (the Raspberry Pi
FLXC1000 uses `0x1000`, so both can share a network). With the CDA checked out
and built in `~/dev/classic-diagnostic-adapter` (or `CDA_DIR`):

```bash
scripts/run-cda.sh <board-ip>     # loads odx/AZ3166.mdd, tester = local interface to the board

B=http://localhost:20002/vehicle/v15
TOKEN=$(curl -s -X POST $B/authorize -H 'Content-Type: application/json' \
  -d '{"client_id":"test","client_secret":"test"}' | jq -r .access_token)
curl -s -H "Authorization: Bearer $TOKEN" $B/components/az3166/data/AmbientTemperature
```

The CDA and the ECU each bind UDP 13400, so `host_sim` and the CDA cannot
share one address; give `host_sim` a second IP (e.g. a `127.0.0.2` loopback
alias) when testing without the board.

## Develop without the board

The ECU and DoIP crates run on the host:

```bash
cargo test                                          # unit tests (DoIP + ECU vs. spec)
cargo run -p flxc1000-ecu --example host_sim        # DoIP ECU on 127.0.0.1:13400
scripts/doip-smoke.py 127.0.0.1 --to-boot
```

`host_sim` uses the same ECU and DoIP code with simulated sensor values, so a
tester or the CDA (with `odx/AZ3166.mdd`) can be pointed at it.

## Differences from flxc1000-rpi

- Transport: DoIP over Wi-Fi (NetX Duo) instead of Ethernet (Linux sockets).
- Variant switch: flash boot-state log instead of `/data/boot_state` + `execve`;
  completing a download (`37`) marks the App valid, so the cycle can be closed
  without reflashing.
- More DIDs (sensors, buttons, RGB, display, network, uptime), sensor DTCs.
- Fixes found while porting (also present upstream / in ace at the pinned rev):
  handler errors now produce NRCs, ECUReset rejects unsupported reset types
  before answering positively, `34`/`36`/`37` require SecurityAccess,
  TransferData no longer echoes the block counter twice, TCP frames split
  across reads are reassembled, generic DoIP header NACKs.

## Licensing

Own code: Apache-2.0. Files derived from the Azure RTOS getting-started guides
(`platform/linker/az3166.ld`, `board.c/.h`, `msp.c`, `net.c`) keep their MIT
notice. Nothing third-party is vendored: `third_party/` is fetched. The WICED
Wi-Fi library and BCM43362 firmware are Cypress property, licensed for use with
Cypress chips only (the AZ3166's EMW3166 module).

> As with the original FLXC1000, most of this code was written with AI
> assistance. It has been tested on the host; check behaviour on your board.

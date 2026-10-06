<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# AZ3166 — ECU Simulator on the MXCHIP AZ3166

> [!WARNING]
> **Disclaimer: this project is 100% AI-generated.** All code, scripts, ODX
> data and documentation in this repository, including this README, were
> generated with [Claude Code](https://claude.com/claude-code) using
> **Claude Opus 5.5**. It has been tested on the host but has not been
> reviewed to production standards. Check the behaviour on your board, and do
> not use it in a real vehicle or any safety-related context.

UDS-over-DoIP ECU simulator running bare-metal on the
[MXCHIP AZ3166 IoT DevKit](https://github.com/MXCHIP/IoTDevKit)
(STM32F412RG Cortex-M4F, 1 MiB flash, 256 KiB RAM, Wi-Fi) with
[Eclipse ThreadX](https://github.com/eclipse-threadx/threadx) and NetX Duo.
The ECU logic is Rust (`no_std`) on top of
[ace-server](https://github.com/samp-reston/ace).

## Based on

This project builds on two base projects:

| Project | What it provides here |
|---------|-----------------------|
| [eclipse-threadx/threadx](https://github.com/eclipse-threadx/threadx) | The RTOS kernel. The firmware runs its threads (UDS worker, DoIP TCP/UDP, routine) on ThreadX. Its sibling [NetX Duo](https://github.com/eclipse-threadx/netxduo) provides the IP stack. Both are pinned in `scripts/fetch-deps.sh` and fetched, not vendored. |
| [theswiftfox/flxc1000-rpi](https://github.com/theswiftfox/flxc1000-rpi) | The original FLXC1000 ECU simulator for the Raspberry Pi 4 (Buildroot Linux). This project is an independent fork. It keeps the same ECU identity, services and Boot/App variant concept, but runs on a microcontroller, and the diagnostic data comes from the board's real sensors and actuators. |

Other upstream code is credited where it is used:
[ace](https://github.com/samp-reston/ace) (UDS server),
the archived [Azure RTOS getting-started guides](https://github.com/azure-rtos/getting-started)
(AZ3166 board support, STM32CubeF4 HAL, WICED Wi-Fi library), and the
[Eclipse OpenSOVD Classic Diagnostic Adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter)
test container, which the ODX generator follows.

The diagnostic surface is specified in [`docs/diagnostics.md`](docs/diagnostics.md);
the matching ODX/PDX/MDD for the Classic Diagnostic Adapter is in [`odx/`](odx/).

## Architecture

```
┌─────────────────────── Rust (crates/) ───────────────────────┐
│ az3166-fw     staticlib: ThreadX entry points, Board impl,   │
│               display screens                                │
│ az3166-ecu    App/Boot variants, DIDs, DTCs, routine,        │  host-tested
│               update package streaming                       │
│ az3166-doip   ISO 13400 DoIP entity, sans-IO                 │  host-tested
│ ace-server    UDS server (no_std)                            │
├────────────────────── C (platform/) ─────────────────────────┤
│ main.c        threads, UDS worker queue, trial confirmation  │
│ bootloader.c  slot selection, rollback, jump to the app      │
│ boot_state.c  persistent boot/update state (flash log)       │
│ update.c      erase / program / commit the inactive slot     │
│ net.c         WICED Wi-Fi, NetX Duo DHCP, DoIP sockets       │
│ board.c       clocks, LEDs, RGB PWM, buttons, sensors, OLED  │
├──────────────────── third_party/ (fetched) ──────────────────┤
│ ThreadX · NetX Duo · STM32CubeF4 HAL · AZ3166 BSP · WICED    │
└──────────────────────────────────────────────────────────────┘
```

| Thread | Prio | Stack | Role |
|--------|------|-------|------|
| `uds` | 4 | 20 KiB | builds the ECU, brings up Wi-Fi, then executes every UDS request (only thread touching the ECU) |
| `tcp0`, `tcp1` | 6 | 3 KiB | DoIP TCP connections (2 concurrent testers) |
| `udp` | 6 | 2 KiB | vehicle announcement / identification |
| `routine` | 8 | 2 KiB | watchdog, SelfTest LED cascade, display screens and buttons |

### Bootloader, app slots and rollback

The firmware is split into a **bootloader** (never updated in the field) and
the **app**, which can be updated through the CDA. The app exists in two
slots, so the previous version stays available for rollback.

| Flash | Size | Content |
|-------|------|---------|
| `0x08000000` | 172 KiB | bootloader: Boot variant (DoIP programming) and slot selection |
| `0x0802B000` | 212 KiB | Wi-Fi chip firmware, used by the bootloader and both app slots |
| `0x08060000` | 256 KiB | app slot A |
| `0x080A0000` | 256 KiB | app slot B |
| `0x080E0000` | 128 KiB | boot/update state log |

On every reset the bootloader decides what runs:

1. Button **B** held, or the App requested the bootloader (`11 01`): **Boot variant**.
2. A freshly installed slot is **on trial**: start it (watchdog on). The app
   **confirms** the slot once Wi-Fi is up and DoIP listens.
3. A trial that hangs, crashes (watchdog, ~16 s) or does not come up within
   120 s is retried; after **3 failed starts** it is rejected and the
   **previous slot runs again** (rollback).
4. Otherwise: start the active (confirmed) slot. No valid app: Boot variant.

An update always goes to the slot that is not active.

```
App ──11 01──▶ Boot ──34/36/37 + 11 01──▶ new App (trial) ──up──▶ confirmed
                                               └──fails 3×──▶ previous App
```

### Board mapping

| Board | Diagnostic use |
|-------|----------------|
| HTS221, LPS22HB, LSM6DSL, LIS2MDL | DIDs `F201`–`F206`; init failures → DTCs `C1xx00` |
| Buttons A/B | previous / next display screen; DID `F210`; B held at reset → Boot variant |
| User LED, Azure LED, RGB R/G/B | SelfTest LED bar (routine `1001`); RGB via DID `F211` |
| Wi-Fi LED | on = DHCP address acquired |
| OLED | firmware version and one screen per readable DID (see below) |
| LSM6DSL accelerometer | turns the display content by 180° when the board is held upside down |
| NAU88C10 codec, headphone jack | routine `1002` speaks the temperature (connect headphones or a speaker) |
| STM32 unique ID / RNG | DID `F18C` / SecurityAccess seed |

### Display

```
App 0.2.0 A          variant, firmware version, running slot
3/17 Pressure        screen number and name
973.0 hPa            value
                     second value line if needed
```

Button **A** shows the previous screen, button **B** the next one; after the
last screen it starts over at the first (and the other way round). The App
has a screen for the network and for every readable DID except the buttons
(temperature, humidity, pressure, acceleration, angular rate, magnetic
field, RGB LED, display text, IP, MAC, uptime, VIN, session, variant, serial
number, software version); the bootloader shows network, session, variant,
serial number and version. Values refresh every second. Writing the RGB LED
or the display text through UDS switches to that screen.

On the **Volume** screen, holding a button changes the volume instead (see
[Temperature announcement](#temperature-announcement)).

When the board is tilted upside down (more than ~0.5 g on the
accelerometer's display axis), the display content turns by 180° so it stays
readable and the buttons swap roles (the left button always goes back);
lying flat keeps the current orientation. The axis and its sign are
`ORIENTATION_AXIS` / `UPRIGHT_SIGN` in `crates/az3166-fw/src/screens.rs`.

## Build

Prerequisites: Rust (rustup), CMake ≥ 3.20, Ninja, git, Python 3.
The Rust target (`thumbv7em-none-eabihf`) is installed automatically through
`rust-toolchain.toml`.

```bash
scripts/fetch-deps.sh        # ThreadX, NetX Duo, AZ3166 BSP -> third_party/
scripts/setup-toolchain.sh   # Arm GNU Toolchain 14.3 -> .toolchain/ (or use your own on PATH)

WIFI_SSID=MyNetwork WIFI_PASSWORD=secret scripts/build.sh
```

| Output (`build/`) | Use |
|--------|-----|
| `az3166-factory.bin` | 1 MiB image for the first flash over USB: bootloader + app in slot A, update state reset |
| `update/az3166-app-<version>.bin` | app update package for the CDA (app linked for slot A and slot B) |
| `az3166-boot.elf`, `az3166-app-a.elf`, `az3166-app-b.elf` | debug symbols |

Credentials are compiled in (WPA2-PSK; empty password = open network) and
stay in `build/CMakeCache.txt`. The app version comes from the CMake cache
variable `APP_VERSION` (default `0.2.0`):

```bash
cmake -S platform -B build -DAPP_VERSION=0.3.0 && cmake --build build
```

`fetch-deps.sh` pins ThreadX and NetX Duo to the exact commits the prebuilt
WICED Wi-Fi library was built against: the struct layouts must match.

## First flash (USB)

Connect the board via USB. `flash.sh` writes `build/az3166-factory.bin`
through the on-board ST-LINK with [OpenOCD](https://openocd.org)
(`brew install open-ocd`, `apt install openocd`):

```bash
scripts/flash.sh             # OpenOCD over SWD, verifies and resets
scripts/flash.sh --drive     # alternative: copy to the AZ3166 USB drive
```

This writes the bootloader and the app (slot A) and clears the update state.
It is only needed once, or to recover a board; later app versions are
installed through the CDA. The board's `AZ3166` USB drive often disappears
after a copy until the board is replugged, which is why OpenOCD is the
default.

Console: the board's USB serial port, 115200 8N1. The display shows the
version and the network screen; once Wi-Fi is up the ECU sends three vehicle
announcements and answers on UDP/TCP 13400.

```bash
scripts/doip-smoke.py <board-ip>            # App variant checks
```

## Use with the Classic Diagnostic Adapter

The ECU is `AZ3166` at logical address `0x1001` (the Raspberry Pi
FLXC1000 uses `0x1000`, so both can share a network). With the CDA checked out
and built in `~/dev/classic-diagnostic-adapter` (or `CDA_DIR`):

```bash
scripts/run-cda.sh <board-ip>     # loads odx/AZ3166.mdd, serves build/update as flash files

B=http://localhost:20002/vehicle/v15
TOKEN=$(curl -s -X POST $B/authorize -H 'Content-Type: application/json' \
  -d '{"client_id":"test","client_secret":"test"}' | jq -r .access_token)
curl -s -H "Authorization: Bearer $TOKEN" $B/components/az3166/data/AmbientTemperature
```

## Temperature announcement

Routine `1002` AnnounceTemperature speaks the ambient temperature on the
headphone jack (connect headphones or an active speaker), e.g. "twenty three
point five degrees celsius". It works in the Default session:

```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  $B/components/az3166/locks -d '{"lock_expiration": 300}'
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  $B/components/az3166/operations/AnnounceTemperature/executions -d '{}'
```

The volume starts at 100 % after every reset and changes in 10 % steps:

| Control | How |
|---------|-----|
| Routines `1003` VolumeUp / `1004` VolumeDown | `POST $B/components/az3166/operations/VolumeUp/executions -d '{}'` (synchronous, returns the new volume) |
| DID `F213` AudioVolume | read in any session; write 0–100 in the Extended session |
| Volume screen | hold the next button (B) for louder, the previous button (A) for quieter (first step after 0.6 s, then every 0.3 s); a short tap still pages |

The codec's headphone output has no analog volume, so the volume is the DAC
digital gain (register `0x0B`): −50 dB at 1 % up to 0 dB at 100 %, 0 % mutes.

The words are pre-recorded clips (zero–nineteen, the tens, hundred, minus,
point, degrees, celsius): 4-bit IMA ADPCM at 8 kHz, 66 KB in the app image
(`platform/assets/speech_words.bin`). They were generated with the macOS
speech synthesizer by `scripts/make-speech.py`; run it on a Mac to change the
voice or the words. The codec (I2C `0x1A`) is fed by I2S2 with circular DMA
(`platform/src/audio.c`).

## Update the app through the CDA

With the CDA running (`scripts/run-cda.sh <board-ip>`) and a new package built:

```bash
cmake -S platform -B build -DAPP_VERSION=0.3.0 && cmake --build build
scripts/az3166-flash                 # newest build/update/*.bin
scripts/az3166-flash path/to/az3166-app-0.3.0.bin
```

`az3166-flash` (Python, standard library only) runs the whole sequence through
the CDA's SOVD API and shows the progress:

| Step | SOVD request | UDS |
|------|--------------|-----|
| lock | `POST /components/az3166/locks` | – |
| enter bootloader (if the App runs) | `POST .../operations/reset/executions` `{"parameters":{"value":"hardreset"}}` | `11 01` |
| programming session | `PUT .../modes/session` `{"value":"programming"}` | `10 02` |
| security access | `PUT .../modes/security` `Level_3_RequestSeed`, then `Level_3` + key (seed XOR `0xDEADBEEF`) | `27 03` / `27 04` |
| list flash files | `GET /apps/sovd2uds/bulk-data/flashfiles` | – |
| request download | `PUT .../x-sovd2uds-download/requestdownload` `{"requestdownload":{"MemoryAddress":134610944,"MemorySize":<size>}}` | `34 00 44 08060000 <size>` (erases the inactive slot) |
| transfer | `POST .../x-sovd2uds-download/flashtransfer` `{"blocksequencecounter":1,"blocksize":4093,"offset":0,"length":<size>,"id":"<file id>"}` | `36` × n |
| transfer exit | `PUT .../x-sovd2uds-download/transferexit` | `37` (verifies the CRC, marks the slot on trial) |
| start new app | reset as above | `11 01` |

Afterwards the script waits until the App is back and checks that it reports
the new version (DID `F195`). If the new app does not come up, the bootloader
rolls back after three attempts and the script reports the old version.

To see the rollback, build an app that hangs at startup and install it:

```bash
cmake -S platform -B build -DAPP_VERSION=0.9.9-hang -DAPP_TEST_HANG=ON && cmake --build build
scripts/az3166-flash build/update/az3166-app-0.9.9-hang.bin
# console: three "APP_TEST_HANG" starts (watchdog resets), then the previous
# app again; az3166-flash reports the rollback
cmake -S platform -B build -DAPP_TEST_HANG=OFF
```

Notes:

- The package contains the app linked for both slots; the bootloader writes
  the image for the inactive slot and checks its CRC-32 at `37`.
- `MemoryAddress` must be `0x08060000` (`134610944`, the app area) and
  `MemorySize` the package size; the slot is chosen by the bootloader.
- The erase stalls the board for 2–4 s. The ECU answers `7F 34 78` first and
  the MDD raises the CDA's `CP_P6Star` to 8 s, so the CDA waits.
- The bootloader is never written over DoIP. Use the factory image over USB
  to change it.

## Develop without the board

The ECU and DoIP crates run on the host:

```bash
cargo test                                          # unit tests (DoIP + ECU vs. spec)
cargo run -p az3166-ecu --example host_sim        # DoIP ECU on 127.0.0.1:13400
scripts/doip-smoke.py 127.0.0.1 --to-boot
```

`host_sim` uses the same ECU and DoIP code with simulated sensor values and
a simulated app slot, so a tester or the CDA (with `odx/AZ3166.mdd`) can be
pointed at it. The CDA and the ECU each bind UDP 13400, so give `host_sim` a
second address (e.g. a `127.0.0.2` loopback alias) when running both on one
machine.

## Differences from flxc1000-rpi

- Transport: DoIP over Wi-Fi (NetX Duo) instead of Ethernet (Linux sockets).
- Real software update: separate bootloader and two app slots with watchdog
  supervised trial and rollback, instead of `/data/boot_state` + `execve`.
- More DIDs (sensors, buttons, RGB, display, network, uptime), sensor DTCs.
- Fixes found while porting (also present upstream / in ace at the pinned rev):
  handler errors now produce NRCs, ECUReset rejects unsupported reset types
  before answering positively, `34`/`36`/`37` require SecurityAccess,
  TransferData no longer echoes the block counter twice, TCP frames split
  across reads are reassembled, generic DoIP header NACKs.

## Licensing

Own code: Apache-2.0. Files derived from the Azure RTOS getting-started guides
(`platform/linker/sections.ld`, `platform/src/board.c/.h`, `msp.c`, `net.c/.h`) keep their MIT
notice. `vendor/ace-server` is ace-server (Apache-2.0, `vendor/ace-server/LICENSE`)
with a small patch (`PATCHES.md`); everything else in `third_party/` is fetched. The WICED
Wi-Fi library and BCM43362 firmware are Cypress property, licensed for use with
Cypress chips only (the AZ3166's EMW3166 module). The word clips in
`platform/assets/speech_words.bin` are voice output of Apple's macOS speech
synthesizer and subject to Apple's license terms, not Apache-2.0 (see
`speech_words.bin.license`); regenerate them with `scripts/make-speech.py`.

## AI disclaimer

This repository is 100% AI-generated with
[Claude Code](https://claude.com/claude-code) using Claude Opus 5.5. A human
directed the work but did not write the code. Treat it
as an experimental, unreviewed project.

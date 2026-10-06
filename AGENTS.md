<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# AGENTS.md — working on the AZ3166 ECU

Project knowledge for coding agents. The README is for humans; this file
collects the non-obvious facts, pitfalls and rules learned while building and
running the project on real hardware.

## What this is

A UDS-over-DoIP ECU simulator on the MXCHIP AZ3166 IoT DevKit (STM32F412RG,
Cortex-M4F, 1 MiB flash, 256 KiB RAM, Wi-Fi via EMW3166/BCM43362), running
ThreadX + NetX Duo, with the ECU logic in Rust (`no_std`) on
[ace-server](https://github.com/samp-reston/ace). Independent fork of
[flxc1000-rpi](https://github.com/theswiftfox/flxc1000-rpi); it is used with
the Eclipse OpenSOVD Classic Diagnostic Adapter (CDA).

- ECU name `AZ3166` (CDA component `az3166`), logical address `0x1001`
  (`0x1000` is the Raspberry Pi FLXC1000), tester `0x0E00`, functional `0xFFFF`.
- VIN `AZ3166ECU00000001` (17 chars), variants `AZ3166_App` (`F100` = `00 01 01`)
  and `AZ3166_Boot` (`FF 00 00`).
- `docs/diagnostics.md` is the single source of truth for the diagnostic
  surface. Change it first, then the firmware (`crates/az3166-ecu`) and the ODX
  (`odx/`), and keep all three consistent.

## Repository layout

| Path | Content |
|------|---------|
| `crates/az3166-doip` | sans-IO ISO 13400 DoIP entity (host-tested) |
| `crates/az3166-ecu` | App/Boot variants, DIDs, DTCs, routines, update package parser, speech words (host-tested); `examples/host_sim.rs` |
| `crates/az3166-fw` | staticlib: C entry points (`az3166_*`), `Board` impl, display screens, DoIP tasks |
| `platform/` | C: `main.c` (threads, UDS worker), `bootloader.c`, `boot_state.c`, `update.c`, `net.c`, `board.c`, `audio.c`, linker scripts, CMake |
| `odx/` | ODX generator (Python + odxtools 11.0.0), `AZ3166.pdx`/`.mdd` (checked in) |
| `scripts/` | `build.sh`, `flash.sh`, `az3166-flash` (CDA update CLI), `run-cda.sh`, `doip-smoke.py`, `make-images.py`, `make-speech.py`, `fetch-deps.sh`, `setup-toolchain.sh` |
| `third_party/`, `.toolchain/`, `build/`, `target/` | fetched / generated, ignored |

## Build and test

```bash
scripts/fetch-deps.sh && scripts/setup-toolchain.sh   # once
WIFI_SSID=... WIFI_PASSWORD=... scripts/build.sh       # credentials end up in build/CMakeCache.txt
cmake -S platform -B build -DAPP_VERSION=0.8.0 && cmake --build build
cargo test                                             # host tests (doip + ecu)
cargo clippy --all-targets
cargo clippy --release --target thumbv7em-none-eabihf -p az3166-fw
```

- Never put Wi-Fi credentials into tracked files.
- `default-members` excludes `az3166-fw`; it only builds for `thumbv7em-none-eabihf`
  (installed via `rust-toolchain.toml`). CMake runs cargo itself.
- `third_party/`: ThreadX and NetX Duo are pinned to the commits the prebuilt
  WICED library (`libwiced_sdk_bin.a`, from the archived azure-rtos
  getting-started repo) was built against, using its `tx_user.h`/`nx_user.h`.
  Do not bump them: struct layouts must match the blob.
- The pinned NetX/ThreadX need `CMAKE_POLICY_VERSION_MINIMUM 3.5` with CMake 4.
- Outputs: `build/az3166-factory.bin` (1 MiB, USB/SWD), `build/update/az3166-app-<version>.bin`
  (CDA package), `az3166-boot.elf`, `az3166-app-{a,b}.elf`.
- `cargo fmt` reformats the big `Screen { .. }` tables in `screens.rs` to
  multi-line; scripted edits must match the formatted text.

## Flashing

- `scripts/flash.sh` uses **OpenOCD over the on-board ST-LINK** (SWD). The
  board's `AZ3166` USB mass-storage drive disappears after a copy until the
  board is replugged; `--drive` still copies to it.
- Factory image = bootloader + app in slot A + erased slot B and state sector
  (resets the update state). The bootloader is only ever changed this way.
- App updates go through the CDA: `scripts/az3166-flash [package]`.
- Console: `/dev/cu.usbmodem*` (USART6, 115200 8N1). `pyserial` is not
  installed; read it with `termios` (open, set B115200 raw, read).

## Flash layout, bootloader, rollback

| Address | Size | Content |
|---------|------|---------|
| `0x08000000` | 172 KiB | bootloader code |
| `0x0802B000` | 212 KiB | BCM43362 Wi-Fi firmware at a fixed address (data `0x0802B000`, handle `0x0805F2E4`) |
| `0x08060000` | 256 KiB | app slot A |
| `0x080A0000` | 256 KiB | app slot B |
| `0x080E0000` | 128 KiB | boot/update state log (sector 11) |

- The apps do **not** contain the 209 KB Wi-Fi firmware; `app_{a,b}.ld`
  define its symbols at the bootloader's addresses (`layout.ld`, asserted in
  `boot.ld`). Changing the bootloader's blob placement breaks all apps.
- App images are linked per slot (execute in place); the update package
  carries both and the bootloader writes the one for the inactive slot.
- Image header `image_info_t` at slot + `0x200` (magic `AZAP`, link base, version).
- State log: append-only 32-bit records `0xA5<type><arg>` (run app/boot,
  installed, attempt, confirmed, rejected); compacted when full. No records +
  valid slot A = factory state.
- Boot decision happens **before any clock setup** (the app must start from
  reset state). The app sets `SCB->VTOR` first thing (SystemInit points it at
  `0x08000000`).
- Trial: new slot gets `INSTALLED`; bootloader counts `ATTEMPT`s and starts
  it with the IWDG on (~16 s, kicked by the lowest-priority `routine` thread).
  The app writes `CONFIRMED` once DoIP listens; not up within 120 s resets.
  3 failed attempts -> `REJECTED`, previous slot runs (rollback). Test with
  `-DAPP_TEST_HANG=ON`.
- Button B held at reset forces the Boot variant.

## Firmware architecture

- Threads: `uds` (prio 4, 20 KiB: ECU init, Wi-Fi bring-up, then every UDS
  request — the only thread touching the ECU), `tcp0/1` (6, 3 KiB), `udp`
  (6, 2 KiB), `routine` (8, 2 KiB: watchdog, self test, screens/buttons).
  DoIP threads hand requests to the worker via a ThreadX queue
  (`plat_uds_execute`).
- RAM is ~97 % used. The ECU static is ~136 KiB because ace-server's outbox
  is 16 x 4 KiB and `drain_outbox` needs a same-sized buffer. `on_read_did`
  needs ~8.5 KiB stack, `az3166_init` ~14.6 KiB. Check stack frames with
  `arm-none-eabi-objdump` before adding large locals.
- State shared outside the worker lives in `Shared` (atomics / `SeqBytes`
  seqlocks; single writer, readers retry — no locks that could invert priorities).
- Display reads use `az3166_ecu::read_did` directly (not through ace-server),
  so polling does not refresh the S3 session timer.
- `HAL_GetTick`/uptime come from the DWT cycle counter (ThreadX owns SysTick).
- Heap: fixed 16 KiB region (`_sbrk` bounded), newlib locks retargeted to one
  ThreadX mutex (newlib's `lock.o` is not weak; define all its symbols).

## ace-server gaps (pinned rev 0706bef), handled in `az3166-ecu`

- Handler errors are returned as `Err(ServerError::Handler)`, not queued as
  NRC -> `serve()` builds the `7F` response.
- ECUReset answers positively **before** calling the handler -> reset types are
  pre-checked in `Policy`.
- `ServiceConfig::secured` is not enforced -> `Policy.secured_sids`.
- Unconfigured services answer `7F` -> `Policy.supported_sids` gives `11`.
- TransferData: ace echoes the block counter itself; the handler must return 0 bytes.

## Board hardware facts

- I2C1 (PB8 SCL / PB9 SDA, 400 kHz) is shared by the sensors (HTS221,
  LPS22HB, LSM6DSL, LIS2MDL), the SSD1306 OLED (`0x3C`) and the NAU88C10 codec
  (`0x1A`). Serialize with `board_i2c_lock()`.
- **The I2C handle is configured with 10-bit own-address mode**, so
  `HAL_I2C_Master_Transmit` addresses targets wrongly. Use `HAL_I2C_Mem_Write`/
  `Mem_Read` (always 7-bit). A wrong transfer can leave the codec holding SDA
  low; `I2C1_BusRecover()` (9 SCL pulses + STOP + peripheral reset) runs at
  startup. An MCU reset alone does not free the bus.
- Codec: NAU88C10, mono. Headphone jack = MOUT (no analog volume, only mute).
  Volume = DAC digital gain reg `0x0B` (`v ? v + 155 : 0` for 0–100 %, max 0 dB).
  Codec register write = `[reg << 1 | bit8][bits 7..0]`. Accessed at 100 kHz
  (temporarily re-init I2C). ID reg `0x40` = `0x0CA`.
- Audio: I2S2 (PB12 WS, PB13 CK, PB15 SD, PC6 MCK, AF5), MCU master, PLLI2S
  M=26 N=344 R=7 -> 8 kHz with I2SDIV 12. DMA1 Stream 4 ch 0 circular,
  refilled in the half/full IRQ (prio 6, no ThreadX calls). I2S and DMA are
  register-level (the BSP's HAL config does not enable the I2S module). The
  devkit SDK's 44.1 kHz PLL values are wrong for the 26 MHz HSE.
- Display: 128x64, `Font_7x10`, 4 lines x 18 chars. 180° rotation =
  commands `0xC0`/`0xA0` (normal `0xC8`/`0xA1`). Orientation from the
  accelerometer: axis 1 (Y), upright sign -1, 500 mg hysteresis.
- Buttons A (PA4), B (PA10), active low, polled. A = previous, B = next;
  swapped when the display is upside down. On the Volume screen a long press
  changes the volume.
- LED bar: user LED, Azure LED, RGB R/G/B (TIM3/TIM2 PWM). Wi-Fi LED = DHCP done.

## Speech

- Temperature announcement (routine `1002`) plays pre-recorded word clips
  (`platform/assets/speech_words.bin`, IMA ADPCM 8 kHz, 66 KB in each app
  image), word ids in `speech_words.h` / `az3166-ecu/src/speech.rs` (same order).
- Clips come from `scripts/make-speech.py` (macOS `say`, voice Samantha). The
  audio is Apple voice output, **not Apache-2.0** (`speech_words.bin.license`).
- Free-form TTS and a PCM upload service were explicitly dropped.

## CDA usage

- `scripts/run-cda.sh <board-ip>`: CDA from `~/dev/classic-diagnostic-adapter`
  (`CDA_DIR`), MDD from `odx/`, flash files from `build/update`, tester
  address = local interface routing to the board, protocol `UDS_Ethernet_DoIP`
  (laptop side), API on `127.0.0.1:20002`. Restart it after regenerating the MDD.
- Token: `POST /vehicle/v15/authorize {"client_id":"test","client_secret":"test"}`.
  Writes, sessions and operations need a lock: `POST .../locks {"lock_expiration": 300}`.
- Session: `PUT .../modes/session {"value":"extended"}`. Security:
  `{"value":"Level_3_RequestSeed"}` -> `seed.Request_Seed` ("0x.. 0x.."), then
  `{"value":"Level_3","Key":{"Send_Key":"0x.. .."}}` (key = seed XOR `0xDEADBEEF`).
- DID writes wrap the value in the parameter name:
  `{"data":{"RgbLedColor":{"Red":0,"Green":0,"Blue":255}}}`.
- Operations: a routine with Stop or RequestResults runs **asynchronously**
  (202, execution must be DELETEd, which sends Stop); Start-only routines run
  synchronously (200). Volume routines are Start-only on purpose.
- Raw UDS: `PUT .../genericservice` with `Content-Type` **and** `Accept:
  application/octet-stream`.
- Flashing: `GET /apps/sovd2uds/bulk-data/flashfiles` must run first (fills
  the file cache), then `requestdownload {"requestdownload":{"MemoryAddress":
  134610944,"MemorySize":<size>}}` (`0x08060000`), `flashtransfer`
  (`blocksize` 4093 = 4095 - SID - counter), `transferexit`. Address and size
  come only from the client.
- Timeouts: CDA defaults P6Max/P6Star 1 s; the MDD sets 2 s / 8 s and RC78
  completion 30 s. The slot erase stalls the CPU 2–4 s (single flash bank),
  so the ECU sends `7F 34 78` first and waits 200 ms for it to leave Wi-Fi.
- DID `F195` is 8 characters: versions longer than 8 chars read back truncated.

### Known CDA issues (CDA is correct-ish, the ECU/ODX are right)

- Linear scaling result truncated to an integer (`23.1` -> `23`),
  `compu_lookup_linear` in `cda-core/src/diag_kernel/operations.rs`.
- Negative 16-bit values zero-padded instead of sign-extended
  (`DiagDataValue::new`).
- Bit fields inside structures fail to decode (ButtonState uses top-level bits).
- DoIP ACKs of concurrent requests (e.g. its own TesterPresent) can reach a
  request waiting for its response: "Unexpected DoIP event type in UDS
  response stream". Read-only CLI requests retry.
- An NRC on `36` does not abort the CDA's transfer loop; verification is at `37`.
- After a Wi-Fi rejoin the first CDA request after a reset can fail; retry.

## ODX

- Generated with Python/odxtools (`odx/generate.sh`, venv in `odx/.venv`),
  converted with odx-converter
  (`ODX_CONVERTER_JAR=~/dev/odx-converter/converter/build/libs/converter-all.jar`).
  `verify.py` decodes sample frames; extend it with every new DID/routine.
- `odx/base/*` are copied unchanged from the CDA testcontainer.
- Flash services must be in functional class `flash_download_upload`.

## Testing on hardware

```bash
scripts/doip-smoke.py <board-ip>            # all App DIDs, routine, DTCs
scripts/doip-smoke.py <board-ip> --to-boot  # older flow; download now needs a real package
scripts/az3166-flash                        # full CDA update incl. trial confirmation
```

The development network was a Wi-Fi with DHCP; the board's IP is shown on
the Network screen and the console.

## Copyright and commit rules (mandatory)

- **Every 100% AI-generated file** carries, directly below its
  `SPDX-License-Identifier` line, in its comment syntax:
  `This file is 100% AI-generated (Claude Code, Claude Opus 5.5).`
  Not on: `LICENSE` (teammate), `Cargo.lock`, `odx/base/*` (copied), the
  MIT-derived files (`platform/linker/sections.ld`, `platform/src/board.c/.h`,
  `msp.c`, `net.c/.h`), binaries (use the `.license` sidecar).
- **Every commit** ends with the trailers, in this order:
  ```
  AI-Generated: 100% (Claude Code, Claude Opus 5.5)
  Signed-off-by: Alexander Mohr <alexander.m.mohr@mercedes-benz.com>
  ```
  `Signed-off-by` must be the last line. Commits touching the MIT-derived
  files use `AI-Generated: yes, adapted from third-party code (Claude Code,
  Claude Opus 5.5)`; copied-only commits get no AI trailer. No
  `Co-authored-by` trailers.
- Use `git commit -s -m "<subject>" -m "AI-Generated: ..."` to get the order right.
- Prefix every git command with `GIT_MASTER=1`. Small atomic commits, plain
  imperative English subjects (≤ 72 chars), commits are GPG-signed.
- Run the `commit-pr-checks` compliance agent before treating commits as final.
- Remote: `origin` = `Eclipse-SDV-Hackathon-Chapter-Four/OpenSOVD-CDA`, branch
  `main`. Rewrite history only for unpushed commits unless the user asks; push
  with `--force-with-lease` and push `main` by name (local `backup/*` branches
  stay local).
- Never commit logs, `.DS_Store`, `.vscode/`, credentials or `build/`.

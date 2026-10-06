<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# AZ3166 ECU Diagnostic Specification

This document is the single source of truth for the diagnostic surface of the
firmware. The firmware (`crates/az3166-ecu`) and the ODX description
(`odx/`) both implement exactly this.

All multi-byte values are **big-endian**. "R" = readable, "W" = writable.

## Transport (ISO 13400 DoIP over Wi-Fi)

| Item | Value |
|------|-------|
| UDP / TCP port | 13400 |
| ECU name (ODX / CDA component) | `AZ3166` (`az3166`) |
| ECU logical address | `0x1001` |
| Functional address | `0xFFFF` |
| Default tester address | `0x0E00` |
| VIN (default) | `AZ3166ECU00000001` |
| EID | Wi-Fi MAC address |
| GID | `00 00 00 00 00 00` |
| IP | DHCP |

## Variants

The Boot variant is the bootloader (fixed); the App variant is the
updatable app, stored in one of two slots. The bootloader decides at reset
from the boot/update state log in flash (last 128 KiB sector).

| Variant | DID `F100` | Runs when |
|---------|-----------|-----------|
| Boot (`AZ3166_Boot`) | `FF 00 00` | Boot requested by the App, button **B** held during reset, or no valid app |
| App (`AZ3166_App`) | `00 01 01` | otherwise: the slot on trial, else the active (confirmed) slot |

Transitions:

- App `11 01` (HardReset) → requests Boot, resets → Boot.
- Boot `37` (RequestTransferExit) after a verified download → the written
  slot is on trial and the App is requested. The next `11 01` starts it.
- An app on trial confirms its slot when DoIP is up. If it hangs or faults
  (watchdog), or is not up within 120 s, the bootloader retries; after 3
  failed starts it rejects the slot and runs the previous one (rollback).
- Boot `11 03` (SoftReset) → resets, stays in Boot.

## Sessions

| ID | Name | App | Boot |
|----|------|-----|------|
| `01` | Default | ✓ | ✓ |
| `02` | Programming | – | ✓ |
| `03` | Extended | ✓ | ✓ |

Session timing (P2 / P2*) uses ace-server defaults: P2 = 50 ms, P2* = 5000 ms.
S3 = 5000 ms.

## Services

| SID | Service | App sessions | Boot sessions | Notes |
|-----|---------|-------------|--------------|-------|
| `10` | DiagnosticSessionControl | 01, 03 | 01, 02, 03 | sub-functions = session IDs |
| `11` | ECUReset | 01, 03 | 01, 02, 03 | App: `01` HardReset. Boot: `01` HardReset, `03` SoftReset |
| `14` | ClearDiagnosticInformation | 01, 03 | – | group `FF FF FF` (any group clears all) |
| `19` | ReadDTCInformation | 01, 03 | – | only sub-function `02` reportDTCByStatusMask |
| `22` | ReadDataByIdentifier | 01, 03 | 01, 02, 03 | one DID per request |
| `27` | SecurityAccess | – | 02, 03 | level `03` (seed) / `04` (key) |
| `2E` | WriteDataByIdentifier | 03 | – | |
| `31` | RoutineControl | 01, 03 | – | SelfTest in 03 only (else NRC `22`) |
| `34` | RequestDownload | – | 02 + security `03` | |
| `36` | TransferData | – | 02 + security `03` | |
| `37` | RequestTransferExit | – | 02 + security `03` | |
| `3E` | TesterPresent | 01, 03 | 01, 02, 03 | sub-function `00` |

### SecurityAccess (Boot)

- `27 03` → `67 03 <seed:4>`
- `27 04 <key:4>` with `key = seed XOR 0xDEADBEEF` → `67 04`
- 3 failed attempts → 10 s lockout (NRC `36` / `37`)

### Download (Boot)

The download is an app update package (`scripts/make-images.py`): a 96-byte
header (magic `AZ3166UP`, version, per slot: link address, offset, size,
CRC-32) followed by the app linked for slot A and for slot B.

- `34 00 44 <addr:4> <size:4>`: address must be `08 06 00 00` (app area),
  size the package size. Erases the inactive slot; the ECU answers
  `7F 34 78` (responsePending) first, then `74 20 0F FF`
  (maxNumberOfBlockLength = 4095: up to 4093 data bytes per `36`).
- `36 <bsc> <data...>` → `76 <bsc>`. Counter starts at 1 and wraps
  `FF → 00`; a repeated counter is acknowledged without writing (tester
  retry). Out of sequence: NRC `73`. Package not for this ECU: NRC `31`.
- `37` → `77` once the whole package arrived and the CRC-32 of the written
  image matches; otherwise NRC `72`. The written slot is then on trial.

Tester timing (MDD comparams): `CP_P6Max` 2 s, `CP_P6Star` 8 s (wait after
`78`, covers the 2–4 s erase), `CP_RC78CompletionTimeout` 30 s.

## Data identifiers

### Common (App and Boot)

| DID | Name | Len | Type / scaling | Access |
|-----|------|-----|----------------|--------|
| `F100` | VariantIdentification | 3 | raw bytes (`00 01 01` App, `FF 00 00` Boot) | R |
| `F186` | ActiveDiagnosticSession | 1 | uint8, session ID | R |
| `F18C` | EcuSerialNumber | 12 | STM32 96-bit unique device ID, raw bytes | R |
| `F195` | SoftwareVersion | 8 | ASCII, space-padded (e.g. `0.1.0   `) | R |

### App only

| DID | Name | Len | Type / scaling | Unit | Access |
|-----|------|-----|----------------|------|--------|
| `F190` | VIN | 17 | ASCII | – | R; W in session 03 |
| `F201` | AmbientTemperature | 2 | sint16, physical = raw × 0.1 | °C | R (HTS221) |
| `F202` | RelativeHumidity | 2 | uint16, physical = raw × 0.1 | %RH | R (HTS221) |
| `F203` | AtmosphericPressure | 2 | uint16, physical = raw × 0.1 | hPa | R (LPS22HB) |
| `F204` | Acceleration | 6 | 3 × sint16 (X, Y, Z), factor 1 | mg | R (LSM6DSL) |
| `F205` | AngularRate | 6 | 3 × sint16 (X, Y, Z), physical = raw × 0.1 | dps | R (LSM6DSL) |
| `F206` | MagneticField | 6 | 3 × sint16 (X, Y, Z), factor 1 | mG | R (LIS2MDL) |
| `F210` | ButtonState | 1 | bitfield: bit 0 = button A pressed, bit 1 = button B pressed | – | R |
| `F211` | RgbLedColor | 3 | 3 × uint8 (R, G, B), 0–255 | – | R; W in session 03 |
| `F212` | DisplayText | 16 | ASCII, space-padded, shown on OLED line 3 | – | R; W in session 03 |
| `F213` | AudioVolume | 1 | uint8, 0–100 (0 = mute; DAC gain −50 dB…0 dB) | % | R; W in session 03 |
| `F220` | IpAddress | 4 | 4 × uint8 (dotted quad) | – | R |
| `F221` | MacAddress | 6 | 6 × uint8 | – | R |
| `F230` | OperatingTime | 4 | uint32, seconds since reset | s | R |

Sensor DIDs return NRC `22` (conditionsNotCorrect) if the sensor failed to
initialise.

## Routines (App)

| RID | Name | Sessions | Start `01` | Stop `02` | Results `03` |
|-----|------|----------|-----------|-----------|--------------|
| `1001` | SelfTest | 03 | runs LED cascade (~1.5 s); `71 01 10 01 01` | aborts; `71 02 10 01 03` | `71 03 10 01 <status>` |
| `1003` | VolumeUp | 01, 03 | volume +10 % (max 100); `71 01 10 03 <volume %>` | – | – |
| `1004` | VolumeDown | 01, 03 | volume −10 % (0 = mute); `71 01 10 04 <volume %>` | – | – |
| `1002` | AnnounceTemperature | 01, 03 | speaks the ambient temperature on the headphone jack; `71 01 10 02 01` | stops; `71 02 10 02 03` (after the end: `71 02 10 02 <status>`) | `71 03 10 02 <status>` |

VolumeUp and VolumeDown have Start only (the CDA runs them synchronously). The
volume is 100 % after every reset.

AnnounceTemperature reads the HTS221 temperature, rounded to 0.1 °C, and
speaks it, e.g. "twenty three point five degrees celsius" or "minus four point
zero degrees celsius" (about 2–3 s). Without a working temperature sensor or
audio codec: NRC `22`.

Routine status byte: `00` idle, `01` running, `02` completed, `03` aborted.
Start while running → NRC `24`. Stop while not running → NRC `24`.

LED cascade order (LED bar 1–5): User LED, Azure LED, RGB red, RGB green, RGB
blue. The Wi-Fi LED shows network state and is not part of the bar.

## DTCs (App)

`19 02 <mask>` → `59 02 FF [<dtc:3> <status:1>]...` (availability mask `FF`).

| DTC | Name | Initial status | Set by |
|-----|------|----------------|--------|
| `C10100` | HumidityTemperatureSensorNoResponse | `09` | HTS221 init failed |
| `C10200` | PressureSensorNoResponse | `09` | LPS22HB init failed |
| `C10300` | InertialSensorNoResponse | `09` | LSM6DSL init failed |
| `C10400` | MagnetometerNoResponse | `09` | LIS2MDL init failed |

`14 FF FF FF` clears all DTCs (they are not re-set until the next reset).

## Negative response codes used

`11` serviceNotSupported, `12` subFunctionNotSupported, `13`
incorrectMessageLengthOrInvalidFormat, `22` conditionsNotCorrect, `24`
requestSequenceError, `31` requestOutOfRange, `33` securityAccessDenied, `35`
invalidKey, `36` exceededNumberOfAttempts, `37` requiredTimeDelayNotExpired,
`7E` subFunctionNotSupportedInActiveSession, `7F`
serviceNotSupportedInActiveSession.

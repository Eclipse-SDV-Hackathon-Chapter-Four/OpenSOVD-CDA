<!--
SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->
<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# AZ3166 ODX

This directory holds the ODX 2.2.0 description of the AZ3166 ECU simulator (FLXC1000 fork) on the
MXCHIP AZ3166. It implements exactly `docs/diagnostics.md`, which is the single
source of truth; change the spec first, then this directory.

The approach and toolchain match the Eclipse OpenSOVD Classic Diagnostic Adapter
test container
([`testcontainer/odx`](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/tree/main/testcontainer/odx)):

1. Python scripts build the database with [`odxtools`](https://github.com/mercedes-benz/odxtools)
   (pinned to the same version, `11.0.0`) and write `AZ3166.pdx`.
2. [`odx-converter`](https://github.com/eclipse-opensovd/odx-converter) turns the PDX
   into `AZ3166.mdd`, which the CDA loads.

Both `AZ3166.pdx` and `AZ3166.mdd` are checked in, so you only need to run the
build after changing the database.

## Build

Local build. The first run creates `.venv` (with `uv` if available, otherwise
`python3 -m venv`), then generates the PDX and verifies it:

```sh
./generate.sh
```

Also produce the MDD (needs Java 21 and a built odx-converter):

```sh
ODX_CONVERTER_JAR=<path-to-odx-converter>/converter/build/libs/converter-all.jar ./generate.sh
```

Docker build of the PDX only, the same way as the reference `generate_docker.sh`:

```sh
./generate_docker.sh
```

Manual steps:

```sh
.venv/bin/python generate.py          # -> AZ3166.pdx
.venv/bin/python verify.py            # load back, list services, encode/decode checks
java -jar converter-all.jar convert AZ3166.pdx   # newer odx-converter (subcommands)
java -jar converter-all.jar AZ3166.pdx           # older odx-converter
```

Set `SOURCE_DATE_EPOCH` for a reproducible `ADMIN-DATA` timestamp.

## Layout

| File | Content |
|------|---------|
| `generate.py` | Entry point. Addresses, timing, the three diag layers |
| `comparams.py` | DoIP/UDS comparams on the base variant, per protocol layer |
| `sessions.py` | `Session` state chart, `10 xx` services |
| `security_access.py` | `SecurityAccess` state chart, `27 03` / `27 04` |
| `reset.py` | `11 01` / `11 03` |
| `shared.py` | Common DOPs and the `22` / `2E` / `3E` builders |
| `dids.py` | DID catalogue (1:1 with the spec tables) |
| `dtc_services.py` | DTC catalogue, `19 02`, `14` |
| `routines.py` | `31 01/02/03 10 01` SelfTest |
| `transferdata.py` | `34` / `36` / `37` |
| `units.py`, `metadata.py`, `helper.py` | Units, admin data and functional classes, builders |
| `verify.py` | Loads the PDX back with odxtools and checks it against the spec |
| `base/` | Protocol and comparam-spec files copied unchanged from the reference |

## Model

| Item | Value |
|------|-------|
| ECU / base variant | `AZ3166` |
| ECU variants | `AZ3166_Boot` (`FF 00 00`), `AZ3166_App` (`00 01 01`) |
| Variant detection | `Identification_Read` (`22 F1 00`), out param `Identification` (uint24) |
| Protocols | `UDS_Ethernet_DoIP`, `UDS_Ethernet_DoIP_DOBT` (as in the reference) |
| `CP_UniqueRespIdTable` | logical address `0x1001`, name `AZ3166` |
| `CP_DoIPLogicalGatewayAddress` | `0x1001` (the ECU is its own DoIP entity) |
| `CP_DoIPLogicalFunctionalAddress` | `0xFFFF` |
| `CP_DoIPLogicalTesterAddress` | `0x0E00` |
| `CP_P2Max` / `CP_P2Star` | 50 ms / 5000 ms (values in µs) |

Base variant, shared by both variants: `Default_Start`, `Extended_Start`, `HardReset`,
`TesterPresent`, and reads of F100, F186, F18C, F195.

Boot adds `Programming_Start`, `SoftReset`, `RequestSeed_Level_3` / `SendKey_Level_3`
(sessions 02/03), and `RequestDownload` / `TransferData` / `TransferExit` (session 02
and `Level_3`).

App adds the App DIDs (`_Read`, plus `_Write` in session 03 for F190, F211, F212),
`FaultMem_ReportDTCByStatusMask`, `FaultMem_ClearDTCs` and
`SelfTest_Start` / `_Stop` / `_RequestResults` (session 03).

The state charts live on the base variant, as in the reference, so the CDA can seed
the default session and security state before variant detection. Session and security
restrictions are modelled as `PRE-CONDITION-STATE-REF`s, which the CDA enforces.
Services available in every session of a variant carry no precondition.

## Known CDA limitations

These were seen when loading the MDD into the CDA against a DoIP stub. The ODX is
correct and odxtools decodes it correctly; the issues are on the CDA side:

- **LINEAR scaling is truncated.** The CDA casts the result back to the coded
  base type, so `F201` 231 is reported as `23` rather than `23.1`
  (`cda-core/src/diag_kernel/operations.rs`, `compu_lookup_linear`).
- **No sign extension.** Signed values shorter than 32 bits (`A_INT32`,
  `BIT-LENGTH` 16) are zero-padded instead of sign-extended, so -20 mg is reported as
  65516 (`cda-core/src/diag_kernel.rs`, `DiagDataValue::new`).
- **F210 ButtonState is flat.** It is modelled as two top-level bit parameters
  (`ButtonA`, `ButtonB`) rather than a structure, because the CDA rejects a
  structure whose bit parameters share byte 0.

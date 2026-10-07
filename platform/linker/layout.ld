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
 *
 * Flash layout of the AZ3166 (STM32F412RG, 1 MiB, sectors 4x16K 64K 7x128K).
 * Shared by the bootloader and the app images.
 *
 *   0x08000000  S0-S5   384K  bootloader (code 172K + Wi-Fi chip firmware)
 *   0x0802B000          212K    BCM43362 firmware at a fixed address; the
 *                               app images reference it instead of a copy
 *   0x08060000  S7-S8   256K  app slot A
 *   0x080A0000  S9-S10  256K  app slot B
 *   0x080E0000  S11     128K  boot/update state log
 */

__boot_origin__      = 0x08000000;
__boot_code_length__ = 0x0002B000;
__wifi_fw_origin__   = 0x0802B000;
__wifi_fw_length__   = 0x00035000;

__slot_a_origin__    = 0x08060000;
__slot_b_origin__    = 0x080A0000;
__slot_length__      = 0x00040000;

__boot_state_start__ = 0x080E0000;
__boot_state_end__   = 0x08100000;

/* Wi-Fi chip firmware placement (asserted by the bootloader link). */
__wifi_fw_data__     = 0x0802B000;
__wifi_fw_handle__   = 0x0805F2E4;

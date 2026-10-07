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
 * Bootloader image: Boot variant (DoIP programming) and slot selection.
 * Carries the Wi-Fi chip firmware at a fixed address for the app images.
 */

ENTRY(Reset_Handler)
INCLUDE layout.ld

_Heap_Size  = 16K;  /* malloc: used by the WICED Wi-Fi driver */
_Stack_Size = 4K;   /* main stack: reset, tx_application_define, ISRs */

MEMORY
{
  FLASH  (rx)  : ORIGIN = 0x08000000, LENGTH = 0x2B000
  WIFIFW (r)   : ORIGIN = 0x0802B000, LENGTH = 0x35000
  RAM    (xrw) : ORIGIN = 0x20000000, LENGTH = 256K
}

SECTIONS
{
  /* Listed first so the generic .rodata rule does not take it. */
  .wifi_firmware :
  {
    KEEP(*libwiced_sdk_bin.a:43362A2_bin.c.obj(.rodata.resources_firmware_DIR_43362_DIR_43362A2_bin_data))
    KEEP(*libwiced_sdk_bin.a:43362A2_bin.c.obj(.rodata.resources_firmware_DIR_43362_DIR_43362A2_bin))
  } >WIFIFW

  INCLUDE sections.ld
}

/* The app images hard-code these addresses (layout.ld). */
ASSERT(resources_firmware_DIR_43362_DIR_43362A2_bin_data == __wifi_fw_data__,
       "Wi-Fi firmware data moved: update layout.ld")
ASSERT(resources_firmware_DIR_43362_DIR_43362A2_bin == __wifi_fw_handle__,
       "Wi-Fi firmware handle moved: update layout.ld")

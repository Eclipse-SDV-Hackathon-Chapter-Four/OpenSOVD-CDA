/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * App image linked for slot A. Uses the Wi-Fi chip firmware stored in the
 * bootloader instead of carrying its own copy.
 */

ENTRY(Reset_Handler)
INCLUDE layout.ld

_Heap_Size  = 16K;  /* malloc: used by the WICED Wi-Fi driver */
_Stack_Size = 4K;   /* main stack: reset, tx_application_define, ISRs */

MEMORY
{
  FLASH (rx)  : ORIGIN = 0x08060000, LENGTH = 0x40000
  RAM   (xrw) : ORIGIN = 0x20000000, LENGTH = 256K
}

resources_firmware_DIR_43362_DIR_43362A2_bin_data = __wifi_fw_data__;
resources_firmware_DIR_43362_DIR_43362A2_bin = __wifi_fw_handle__;

SECTIONS
{
  INCLUDE sections.ld
}

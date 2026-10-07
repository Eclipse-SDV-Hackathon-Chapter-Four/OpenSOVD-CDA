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
 * App update (Boot variant): erase the inactive slot, program it, commit it
 * as "on trial". The package parsing and verification live in Rust
 * (crates/az3166-ecu/src/update.rs).
 */

#include <string.h>

#include "stm32f4xx_hal.h"

#include "boot_state.h"
#include "platform.h"

/* Slot sectors: A = 7..8, B = 9..10 (128 KiB each) */
#define SLOT_A_FIRST_SECTOR FLASH_SECTOR_7
#define SLOT_B_FIRST_SECTOR FLASH_SECTOR_9
#define SLOT_SECTORS        2u

static uint8_t target_slot = SLOT_NONE;

uint32_t plat_update_begin(void)
{
    target_slot = boot_state_update_target();

    HAL_FLASH_Unlock();
    __HAL_FLASH_CLEAR_FLAG(FLASH_FLAG_EOP | FLASH_FLAG_OPERR | FLASH_FLAG_WRPERR | FLASH_FLAG_PGAERR |
                           FLASH_FLAG_PGPERR | FLASH_FLAG_PGSERR);
    int erased = flash_erase_sectors(
        target_slot == SLOT_A ? SLOT_A_FIRST_SECTOR : SLOT_B_FIRST_SECTOR, SLOT_SECTORS);
    HAL_FLASH_Lock();

    if (erased != 0)
    {
        target_slot = SLOT_NONE;
        return 0;
    }
    return slot_base(target_slot);
}

int32_t plat_flash_program(uint32_t address, const uint8_t* data, uint32_t len)
{
    if (target_slot == SLOT_NONE || slot_of_address(address) != target_slot ||
        slot_of_address(address + len - 1) != target_slot)
    {
        return -1;
    }

    int32_t result = 0;
    HAL_FLASH_Unlock();
    for (uint32_t i = 0; i < len && result == 0;)
    {
        uint32_t a = address + i;
        if ((a & 3u) == 0 && len - i >= 4)
        {
            uint32_t word;
            memcpy(&word, &data[i], 4);
            result = HAL_FLASH_Program(FLASH_TYPEPROGRAM_WORD, a, word) == HAL_OK ? 0 : -1;
            i += 4;
        }
        else
        {
            result = HAL_FLASH_Program(FLASH_TYPEPROGRAM_BYTE, a, data[i]) == HAL_OK ? 0 : -1;
            i += 1;
        }
    }
    HAL_FLASH_Lock();

    /* Read back: catches writes to non-erased flash. */
    if (result == 0 && memcmp((const void*)address, data, len) != 0)
    {
        result = -1;
    }
    return result;
}

int32_t plat_update_commit(uint32_t base)
{
    uint8_t slot = slot_of_address(base);
    if (slot == SLOT_NONE || slot != target_slot || slot_image(slot) == NULL)
    {
        return -1;
    }
    target_slot = SLOT_NONE;
    if (boot_state_append(REC_INSTALLED, slot) != 0)
    {
        return -1;
    }
    return plat_boot_state_write(PLAT_BOOT_STATE_APP_VALID);
}

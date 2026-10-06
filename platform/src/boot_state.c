/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * Persistent boot state (App / Boot variant selection) in flash sector 11.
 *
 * The sector is an append-only log of 32-bit records, so a state change
 * programs one word (microseconds) instead of erasing 128 KiB (seconds,
 * stalling the CPU). The last valid record wins; the sector is erased only
 * when it is full. Words without the magic (e.g. left by other firmware) are
 * skipped.
 */

#include "boot_state.h"

#include "stm32f4xx_hal.h"

#include "platform.h"

#define RECORD_MAGIC 0xF1C00000u
#define MAGIC_MASK   0xFFFF0000u
#define ERASED       0xFFFFFFFFu
#define BOOT_SECTOR  FLASH_SECTOR_11

extern uint32_t __boot_state_start__[];
extern uint32_t __boot_state_end__[];

uint32_t boot_state_read(void)
{
    uint32_t state = 0;
    for (const uint32_t* p = __boot_state_start__; p < __boot_state_end__; p++)
    {
        uint32_t word = *p;
        if ((word & MAGIC_MASK) == RECORD_MAGIC)
        {
            state = word & ~MAGIC_MASK;
        }
    }
    return state;
}

static uint32_t* next_free_slot(void)
{
    /* First erased word after the last used one. */
    uint32_t* p = __boot_state_end__;
    while (p > __boot_state_start__ && p[-1] == ERASED)
    {
        p--;
    }
    return p < __boot_state_end__ ? p : NULL;
}

static int erase_sector(void)
{
    FLASH_EraseInitTypeDef erase = {0};
    uint32_t sector_error        = 0;

    erase.TypeErase    = FLASH_TYPEERASE_SECTORS;
    erase.Sector       = BOOT_SECTOR;
    erase.NbSectors    = 1;
    erase.VoltageRange = FLASH_VOLTAGE_RANGE_3;
    return HAL_FLASHEx_Erase(&erase, &sector_error) == HAL_OK ? 0 : -1;
}

int32_t plat_boot_state_write(uint32_t state)
{
    if (boot_state_read() == state)
    {
        return 0;
    }

    int32_t result = -1;
    HAL_FLASH_Unlock();
    __HAL_FLASH_CLEAR_FLAG(FLASH_FLAG_EOP | FLASH_FLAG_OPERR | FLASH_FLAG_WRPERR | FLASH_FLAG_PGAERR |
                           FLASH_FLAG_PGPERR | FLASH_FLAG_PGSERR);

    uint32_t* slot = next_free_slot();
    if (slot == NULL)
    {
        if (erase_sector() != 0)
        {
            goto out;
        }
        slot = __boot_state_start__;
    }
    if (HAL_FLASH_Program(FLASH_TYPEPROGRAM_WORD, (uint32_t)slot, RECORD_MAGIC | state) == HAL_OK &&
        *slot == (RECORD_MAGIC | state))
    {
        result = 0;
    }

out:
    HAL_FLASH_Lock();
    return result;
}

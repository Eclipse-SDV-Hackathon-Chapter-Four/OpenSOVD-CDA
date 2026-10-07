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
 * Persistent boot/update state in flash sector 11.
 *
 * The sector is an append-only log of 32-bit records, so a state change
 * programs one word (microseconds) instead of erasing 128 KiB (seconds,
 * stalling the CPU). The state is derived by replaying the log; when the
 * sector is full it is erased and the current state is written back
 * (compaction). Words without the magic (e.g. left by other firmware) are
 * skipped. The app also keeps its alarm configuration here (a bootloader
 * older than these records drops them when it compacts the sector).
 */

#include "boot_state.h"

#include <string.h>

#include "stm32f4xx_hal.h"

#include "platform.h"

#define RECORD_MAGIC 0xA5000000u
#define MAGIC_MASK   0xFF000000u
#define ERASED       0xFFFFFFFFu
#define STATE_SECTOR FLASH_SECTOR_11

#define RECORD(type, arg) (RECORD_MAGIC | ((uint32_t)(type) << 16) | ((uint32_t)(arg) & 0xFFFFu))

extern uint32_t __boot_state_start__[];
extern uint32_t __boot_state_end__[];
extern uint32_t __slot_a_origin__[];
extern uint32_t __slot_b_origin__[];
extern uint32_t __slot_length__[];

uint32_t slot_base(uint8_t slot)
{
    return slot == SLOT_A ? (uint32_t)__slot_a_origin__ : (uint32_t)__slot_b_origin__;
}

uint32_t slot_length(void)
{
    return (uint32_t)__slot_length__;
}

uint8_t slot_of_address(uint32_t address)
{
    for (uint8_t slot = SLOT_A; slot <= SLOT_B; slot++)
    {
        if (address >= slot_base(slot) && address < slot_base(slot) + slot_length())
        {
            return slot;
        }
    }
    return SLOT_NONE;
}

const image_info_t* slot_image(uint8_t slot)
{
    uint32_t base             = slot_base(slot);
    const uint32_t* vectors   = (const uint32_t*)base;
    const image_info_t* info  = (const image_info_t*)(base + IMAGE_INFO_OFFSET);
    uint32_t stack            = vectors[0];
    uint32_t reset            = vectors[1];

    if (info->magic != IMAGE_INFO_MAGIC || info->link_base != base)
    {
        return NULL;
    }
    if (stack < SRAM1_BASE || stack > SRAM1_BASE + 256u * 1024u)
    {
        return NULL;
    }
    if (reset < base || reset >= base + slot_length())
    {
        return NULL;
    }
    return info;
}

void boot_state_get(boot_state_t* state)
{
    memset(state, 0, sizeof(*state));
    state->active     = SLOT_NONE;
    state->trial      = SLOT_NONE;
    state->cfg_rise   = CFG_UNSET;
    state->cfg_window = CFG_UNSET;
    state->cfg_fall   = CFG_UNSET;
    state->cfg_hot    = CFG_UNSET;
    state->cfg_hold   = CFG_UNSET;
    int any_slot_record = 0;

    for (const uint32_t* p = __boot_state_start__; p < __boot_state_end__; p++)
    {
        uint32_t word = *p;
        if ((word & MAGIC_MASK) != RECORD_MAGIC)
        {
            continue;
        }
        uint32_t type = (word >> 16) & 0xFFu;
        uint8_t slot  = (uint8_t)(word & 0xFFFFu);
        if (type >= REC_CFG_RISE && type <= REC_CFG_HOLD)
        {
            uint16_t* cfg[] = {&state->cfg_rise, &state->cfg_window, &state->cfg_fall, &state->cfg_hot,
                &state->cfg_hold};
            *cfg[type - REC_CFG_RISE] = (uint16_t)word;
            continue;
        }
        if (type >= REC_INSTALLED && slot > SLOT_B)
        {
            continue;
        }
        switch (type)
        {
            case REC_RUN_APP:
                state->run_boot = 0;
                break;
            case REC_RUN_BOOT:
                state->run_boot = 1;
                break;
            case REC_INSTALLED:
                any_slot_record      = 1;
                state->status[slot]  = SLOT_TRIAL;
                state->attempts[slot] = 0;
                state->trial         = slot;
                if (state->active == slot)
                {
                    state->active = SLOT_NONE;
                }
                break;
            case REC_ATTEMPT:
                if (state->status[slot] == SLOT_TRIAL)
                {
                    state->attempts[slot]++;
                }
                break;
            case REC_CONFIRMED:
                any_slot_record     = 1;
                state->status[slot] = SLOT_CONFIRMED;
                state->active       = slot;
                if (state->trial == slot)
                {
                    state->trial = SLOT_NONE;
                }
                break;
            case REC_REJECTED:
                any_slot_record     = 1;
                state->status[slot] = SLOT_REJECTED;
                if (state->trial == slot)
                {
                    state->trial = SLOT_NONE;
                }
                if (state->active == slot)
                {
                    state->active = SLOT_NONE;
                }
                break;
            default:
                break;
        }
    }

    /* The active slot falls back to the other confirmed one, if any. */
    if (state->active == SLOT_NONE)
    {
        for (uint8_t slot = SLOT_A; slot <= SLOT_B; slot++)
        {
            if (state->status[slot] == SLOT_CONFIRMED)
            {
                state->active = slot;
            }
        }
    }

    /* Factory image: the app was flashed into slot A together with the
     * bootloader and no update has happened yet. */
    if (!any_slot_record && slot_image(SLOT_A) != NULL)
    {
        state->status[SLOT_A] = SLOT_CONFIRMED;
        state->active         = SLOT_A;
    }
}

uint8_t boot_state_update_target(void)
{
    boot_state_t state;
    boot_state_get(&state);
    return state.active == SLOT_A ? SLOT_B : SLOT_A;
}

/* ---- writing --------------------------------------------------------------- */

static void flash_unlock(void)
{
    HAL_FLASH_Unlock();
    __HAL_FLASH_CLEAR_FLAG(FLASH_FLAG_EOP | FLASH_FLAG_OPERR | FLASH_FLAG_WRPERR | FLASH_FLAG_PGAERR |
                           FLASH_FLAG_PGPERR | FLASH_FLAG_PGSERR);
}

static int program_word(uint32_t* address, uint32_t value)
{
    return HAL_FLASH_Program(FLASH_TYPEPROGRAM_WORD, (uint32_t)address, value) == HAL_OK && *address == value
               ? 0
               : -1;
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

int flash_erase_sectors(uint32_t first_sector, uint32_t count)
{
    FLASH_EraseInitTypeDef erase = {0};
    uint32_t sector_error        = 0;

    erase.TypeErase    = FLASH_TYPEERASE_SECTORS;
    erase.Sector       = first_sector;
    erase.NbSectors    = count;
    erase.VoltageRange = FLASH_VOLTAGE_RANGE_3;
    return HAL_FLASHEx_Erase(&erase, &sector_error) == HAL_OK ? 0 : -1;
}

/* Erases the sector and writes the current state back as fresh records. */
static int compact(void)
{
    boot_state_t state;
    uint32_t records[13];
    uint32_t n = 0;

    boot_state_get(&state);
    for (uint8_t slot = SLOT_A; slot <= SLOT_B; slot++)
    {
        if (slot == state.active)
        {
            continue;
        }
        if (state.status[slot] == SLOT_CONFIRMED)
        {
            records[n++] = RECORD(REC_CONFIRMED, slot);
        }
        else if (state.status[slot] == SLOT_REJECTED)
        {
            records[n++] = RECORD(REC_REJECTED, slot);
        }
    }
    if (state.active != SLOT_NONE)
    {
        records[n++] = RECORD(REC_CONFIRMED, state.active);
    }
    if (state.trial != SLOT_NONE)
    {
        records[n++] = RECORD(REC_INSTALLED, state.trial);
        for (uint8_t i = 0; i < state.attempts[state.trial] && n < 7; i++)
        {
            records[n++] = RECORD(REC_ATTEMPT, state.trial);
        }
    }
    records[n++] = RECORD(state.run_boot ? REC_RUN_BOOT : REC_RUN_APP, 0);
    if (state.cfg_rise != CFG_UNSET)
    {
        records[n++] = RECORD(REC_CFG_RISE, state.cfg_rise);
    }
    if (state.cfg_window != CFG_UNSET)
    {
        records[n++] = RECORD(REC_CFG_WINDOW, state.cfg_window);
    }
    if (state.cfg_fall != CFG_UNSET)
    {
        records[n++] = RECORD(REC_CFG_FALL, state.cfg_fall);
    }
    if (state.cfg_hot != CFG_UNSET)
    {
        records[n++] = RECORD(REC_CFG_HOT, state.cfg_hot);
    }
    if (state.cfg_hold != CFG_UNSET)
    {
        records[n++] = RECORD(REC_CFG_HOLD, state.cfg_hold);
    }

    if (flash_erase_sectors(STATE_SECTOR, 1) != 0)
    {
        return -1;
    }
    for (uint32_t i = 0; i < n; i++)
    {
        if (program_word(&__boot_state_start__[i], records[i]) != 0)
        {
            return -1;
        }
    }
    return 0;
}

int boot_state_append(uint32_t type, uint16_t arg)
{
    int result = -1;
    flash_unlock();

    uint32_t* free_word = next_free_slot();
    if (free_word == NULL)
    {
        if (compact() != 0)
        {
            goto out;
        }
        free_word = next_free_slot();
        if (free_word == NULL)
        {
            goto out;
        }
    }
    result = program_word(free_word, RECORD(type, arg));

out:
    HAL_FLASH_Lock();
    return result;
}

int32_t plat_boot_state_write(uint32_t state)
{
    boot_state_t current;
    boot_state_get(&current);

    uint32_t run_boot = state == PLAT_BOOT_STATE_BOOT_REQUESTED;
    if (current.run_boot == run_boot)
    {
        return 0;
    }
    return boot_state_append(run_boot ? REC_RUN_BOOT : REC_RUN_APP, 0);
}

/* Alarm configuration: rise, window, fall, hot limit, hot hold time (in this
 * order, also the record types REC_CFG_RISE..REC_CFG_HOLD). Load: 1 if the
 * first three are saved; later ones may be CFG_UNSET (saved by an older app). */
int32_t plat_alarm_config_load(uint16_t values[PLAT_ALARM_CONFIG_COUNT])
{
    boot_state_t state;
    boot_state_get(&state);
    if (state.cfg_rise == CFG_UNSET || state.cfg_window == CFG_UNSET || state.cfg_fall == CFG_UNSET)
    {
        return 0;
    }
    values[0] = state.cfg_rise;
    values[1] = state.cfg_window;
    values[2] = state.cfg_fall;
    values[3] = state.cfg_hot;
    values[4] = state.cfg_hold;
    return 1;
}

int32_t plat_alarm_config_store(const uint16_t values[PLAT_ALARM_CONFIG_COUNT])
{
    boot_state_t state;
    boot_state_get(&state);
    const uint16_t saved[PLAT_ALARM_CONFIG_COUNT] = {
        state.cfg_rise, state.cfg_window, state.cfg_fall, state.cfg_hot, state.cfg_hold};
    for (uint32_t i = 0; i < PLAT_ALARM_CONFIG_COUNT; i++)
    {
        if (saved[i] != values[i] && boot_state_append(REC_CFG_RISE + i, values[i]) != 0)
        {
            return -1;
        }
    }
    return 0;
}

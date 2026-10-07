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
 * Persistent boot/update state (see boot_state.c) and app slot helpers.
 */

#ifndef AZ3166_BOOT_STATE_H
#define AZ3166_BOOT_STATE_H

#include <stdint.h>

#include "image_info.h"

#define SLOT_A    0u
#define SLOT_B    1u
#define SLOT_NONE 0xFFu

/* Record types */
#define REC_RUN_APP   1u /* start the app (default) */
#define REC_RUN_BOOT  2u /* stay in the Boot variant (App ECUReset) */
#define REC_INSTALLED 3u /* new image written to slot: on trial */
#define REC_ATTEMPT   4u /* bootloader started the trial slot */
#define REC_CONFIRMED 5u /* app in slot came up: slot is good */
#define REC_REJECTED  6u /* trial failed: rolled back */
#define REC_CFG_RISE   7u /* app: alarm rise threshold, 0.1 degC */
#define REC_CFG_WINDOW 8u /* app: alarm time window, s */
#define REC_CFG_FALL   9u /* app: alarm fall threshold, 0.1 degC */
#define REC_CFG_HOT    10u /* app: alarm hot limit, 0.1 degC */
#define REC_CFG_HOLD   11u /* app: alarm hot hold time, s */

#define CFG_UNSET 0xFFFFu

/* Slot status */
#define SLOT_EMPTY     0u
#define SLOT_TRIAL     1u
#define SLOT_CONFIRMED 2u
#define SLOT_REJECTED  3u

typedef struct
{
    uint8_t run_boot;    /* stay in the Boot variant */
    uint8_t active;      /* last confirmed slot, or SLOT_NONE */
    uint8_t trial;       /* slot on trial, or SLOT_NONE */
    uint8_t status[2];
    uint8_t attempts[2]; /* starts of the trial slot so far */
    uint16_t cfg_rise;   /* app configuration, or CFG_UNSET */
    uint16_t cfg_window;
    uint16_t cfg_fall;
    uint16_t cfg_hot;
    uint16_t cfg_hold;
} boot_state_t;

void boot_state_get(boot_state_t* state);
/* 0 = ok */
/* arg: the slot, or a configuration value */
int boot_state_append(uint32_t type, uint16_t arg);
/* Slot an update is written to: the one not active. */
uint8_t boot_state_update_target(void);

uint32_t slot_base(uint8_t slot);
uint32_t slot_length(void);
uint8_t slot_of_address(uint32_t address);
/* Image header if the slot holds a plausible image linked for it, else NULL. */
const image_info_t* slot_image(uint8_t slot);

/* 0 = ok */
int flash_erase_sectors(uint32_t first_sector, uint32_t count);

#endif

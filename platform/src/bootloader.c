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
 * Slot selection, run by the bootloader image right after reset, before any
 * clock or peripheral setup (the app must start from a clean reset state).
 *
 *   button B held / Boot requested  -> stay: Boot variant
 *   slot on trial, < 3 attempts     -> count attempt, start it (watchdog on)
 *   slot on trial, 3 attempts used  -> reject it: roll back
 *   active slot valid               -> start it (watchdog on)
 *   otherwise                       -> stay: Boot variant
 *
 * The app confirms its slot once it is up (DoIP listening). A trial that
 * crashes or hangs is reset by the watchdog and retried; after the third
 * attempt the previous slot runs again.
 */

#include "bootloader.h"

#include "stm32f4xx.h"

#include "boot_state.h"
#include "watchdog.h"

#define MAX_TRIAL_ATTEMPTS 3u

static int button_b_pressed(void)
{
    RCC->AHB1ENR |= RCC_AHB1ENR_GPIOAEN;
    (void)RCC->AHB1ENR;
    /* PA10, input after reset, active low; give the pin a moment to settle. */
    for (volatile int i = 0; i < 1000; i++)
    {
    }
    int pressed = (GPIOA->IDR & GPIO_IDR_ID10) == 0;
    RCC->AHB1ENR &= ~RCC_AHB1ENR_GPIOAEN;
    return pressed;
}

static void start_slot(uint8_t slot)
{
    uint32_t base   = slot_base(slot);
    uint32_t stack  = ((const uint32_t*)base)[0];
    uint32_t reset  = ((const uint32_t*)base)[1];

    watchdog_start();

    __disable_irq();
    SCB->VTOR = base;
    __DSB();
    __ISB();
    __set_MSP(stack);
    ((void (*)(void))reset)();
    for (;;)
    {
    }
}

void bootloader_select(void)
{
    if (button_b_pressed())
    {
        return;
    }

    boot_state_t state;
    boot_state_get(&state);
    if (state.run_boot)
    {
        return;
    }

    if (state.trial != SLOT_NONE)
    {
        uint8_t trial = state.trial;
        if (state.attempts[trial] < MAX_TRIAL_ATTEMPTS && slot_image(trial) != NULL)
        {
            /* Flash writes before the HAL is initialised work: the flash
             * interface is clocked from reset. */
            if (boot_state_append(REC_ATTEMPT, trial) == 0)
            {
                start_slot(trial);
            }
        }
        boot_state_append(REC_REJECTED, trial);
        boot_state_get(&state);
    }

    if (state.active != SLOT_NONE && slot_image(state.active) != NULL)
    {
        start_slot(state.active);
    }
}

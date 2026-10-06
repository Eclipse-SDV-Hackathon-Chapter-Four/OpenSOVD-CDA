/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * Independent watchdog (IWDG, ~32 kHz LSI): started by the bootloader before
 * it jumps to an app, kicked by the app's lowest-priority thread. An app that
 * hangs or faults is reset, which the bootloader counts as a failed trial.
 * Once started it runs until the next reset, so the Boot variant (which
 * never starts it) is not affected.
 */

#include "watchdog.h"

#include "stm32f4xx.h"

#include "platform.h"

#define IWDG_KEY_ENABLE 0xCCCCu
#define IWDG_KEY_ACCESS 0x5555u
#define IWDG_KEY_RELOAD 0xAAAAu

/* LSI / 128 = 250 Hz; 4095 ticks = ~16 s (LSI varies 17..47 kHz: 11..30 s) */
#define IWDG_PRESCALER_128 5u
#define IWDG_RELOAD        4095u

void watchdog_start(void)
{
    IWDG->KR  = IWDG_KEY_ENABLE;
    IWDG->KR  = IWDG_KEY_ACCESS;
    IWDG->PR  = IWDG_PRESCALER_128;
    IWDG->RLR = IWDG_RELOAD;
    while (IWDG->SR != 0)
    {
    }
    IWDG->KR = IWDG_KEY_RELOAD;
}

void plat_watchdog_kick(void)
{
    IWDG->KR = IWDG_KEY_RELOAD;
}

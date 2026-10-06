/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * The bootloader has no audio: the shared Rust library still links these.
 */

#include "platform.h"

int32_t plat_audio_ok(void)
{
    return 0;
}

uint32_t plat_audio_free(void)
{
    return 0;
}

uint32_t plat_audio_pending(void)
{
    return 0;
}

uint32_t plat_audio_write(const int16_t* samples, uint32_t count)
{
    (void)samples;
    (void)count;
    return 0;
}

void plat_audio_clear(void)
{
}

void plat_audio_set_volume(uint32_t percent)
{
    (void)percent;
}

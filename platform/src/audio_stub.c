/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * The bootloader has no audio: the shared Rust library still links these.
 */

#include "platform.h"

int32_t plat_audio_say(const uint8_t* words, uint32_t len)
{
    (void)words;
    (void)len;
    return 0;
}

int32_t plat_audio_busy(void)
{
    return 0;
}

void plat_audio_stop(void)
{
}

void plat_audio_set_volume(uint32_t percent)
{
    (void)percent;
}

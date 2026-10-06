/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * Header of an app image, placed by the linker at IMAGE_INFO_OFFSET from the
 * slot start (right after the vector table). The bootloader checks it before
 * starting a slot.
 */

#ifndef AZ3166_IMAGE_INFO_H
#define AZ3166_IMAGE_INFO_H

#include <stdint.h>

#define IMAGE_INFO_MAGIC  0x50415A41u /* "AZAP" */
#define IMAGE_INFO_OFFSET 0x200u      /* vector table is 0x1C4 bytes */
#define IMAGE_VERSION_LEN 16u

typedef struct
{
    uint32_t magic;
    uint32_t link_base; /* slot the image was linked for */
    char version[IMAGE_VERSION_LEN]; /* NUL padded */
} image_info_t;

#endif

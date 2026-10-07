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
 * Headphone audio for the app: NAU88C10 codec (I2C1, address 0x1A) fed over
 * I2S2 (PB12 WS, PB13 CK, PB15 SD, PC6 MCK; STM32 is master) by DMA1 Stream 4
 * channel 0 at 8 kHz.
 *
 * The DMA runs continuously in circular mode on a two-half stereo buffer;
 * the half-/full-transfer interrupt refills the half just played from a
 * sample ring buffer (filled by the speech thread), or with silence.
 * Running all the time avoids start/stop clicks.
 *
 * I2S and DMA are driven at register level: the HAL configuration of the
 * board support package does not enable the I2S module.
 */

#include "audio.h"

#include <stdio.h>
#include <string.h>

#include "stm32f4xx_hal.h"

#include "board.h"
#include "platform.h"

#define CODEC_ADDRESS (0x1Au << 1)
#define CODEC_ID_REG  0x40u
#define CODEC_ID      0x0CAu

/* PLLI2S from the 26 MHz HSE: /26 * 344 / 7 = 49.142857 MHz I2S clock.
 * With MCLK output: Fs = 49.142857 MHz / (256 * 24) = 7998.5 Hz. */
#define PLLI2S_M   26u
#define PLLI2S_N   344u
#define PLLI2S_R   7u
#define PLLI2S_Q   4u /* unused, must be valid */
#define I2S_DIV    12u
#define I2S_MASTER_TX 2u

#define FRAMES_PER_HALF 256u /* 32 ms at 8 kHz */
#define RING_SIZE       4096u /* samples, power of two: 0.5 s at 8 kHz */

/* Stereo frames (left, right); the codec plays the left slot. */
static int16_t dma_buffer[2u * FRAMES_PER_HALF * 2u];

static int audio_ok;

/* Mono sample ring: one producer (plat_audio_write), one consumer (the DMA
 * interrupt). Free-running indexes, masked on access. */
static int16_t ring[RING_SIZE];
static volatile uint32_t ring_head; /* written by the producer */
static volatile uint32_t ring_tail; /* written by the interrupt */

/* ---- codec --------------------------------------------------------------- */

/* Register write: [reg << 1 | value bit 8] [value bits 7..0]. Sent with
 * Mem_Write, which always uses 7-bit addressing (Master_Transmit follows the
 * handle's 10-bit own-address setting and would address the codec wrongly). */
static int codec_write(uint8_t reg, uint16_t value)
{
    uint8_t low = (uint8_t)value;
    uint16_t first = (uint16_t)((reg << 1) | ((value >> 8) & 1u));
    return HAL_I2C_Mem_Write(&I2cHandle, CODEC_ADDRESS, first, I2C_MEMADD_SIZE_8BIT, &low, 1, 10) == HAL_OK ? 0
                                                                                                      : -1;
}

static int codec_read(uint8_t reg, uint16_t* value)
{
    uint8_t data[2];
    if (HAL_I2C_Mem_Read(&I2cHandle, CODEC_ADDRESS, (uint16_t)(reg << 1), I2C_MEMADD_SIZE_8BIT, data, 2, 10) !=
        HAL_OK)
    {
        return -1;
    }
    *value = (uint16_t)(((data[0] & 1u) << 8) | data[1]);
    return 0;
}

/* Playback setup of the devkit SDK (nau88c10.c), DAC -> mono mixer (MOUT,
 * headphone jack) and speaker mixer, 16-bit I2S slave, MCLK direct, 8 kHz
 * filters, 0 dB DAC gain. */
static const uint16_t codec_init[][2] = {
    {0x01, 0x15D}, {0x02, 0x015}, {0x03, 0x0ED}, {0x04, 0x010}, {0x05, 0x000}, {0x06, 0x00C},
    {0x07, 0x00A}, {0x08, 0x000}, {0x09, 0x000}, {0x0A, 0x008}, {0x0B, 0x1FF}, {0x0C, 0x000},
    {0x0D, 0x000}, {0x0E, 0x108}, {0x0F, 0x1FF}, {0x12, 0x12C}, {0x13, 0x02C}, {0x14, 0x02C},
    {0x15, 0x02C}, {0x16, 0x02C}, {0x18, 0x032}, {0x19, 0x000}, {0x1B, 0x000}, {0x1C, 0x000},
    {0x1D, 0x000}, {0x1E, 0x000}, {0x20, 0x038}, {0x21, 0x00B}, {0x22, 0x032}, {0x23, 0x000},
    {0x24, 0x008}, {0x25, 0x00C}, {0x26, 0x093}, {0x27, 0x0E9}, {0x28, 0x000}, {0x2C, 0x003},
    {0x2D, 0x010}, {0x2E, 0x000}, {0x2F, 0x100}, {0x30, 0x000}, {0x31, 0x002}, {0x32, 0x001},
    {0x33, 0x000}, {0x34, 0x040}, {0x35, 0x040}, {0x36, 0x0BF}, {0x37, 0x040}, {0x38, 0x001},
    {0x3C, 0x004},
};

/* The codec is accessed at 100 kHz like the devkit SDK does; the bus runs at
 * 400 kHz for the sensors and the display otherwise. */
static void i2c_speed(uint32_t hz)
{
    HAL_I2C_DeInit(&I2cHandle);
    I2cHandle.Init.ClockSpeed = hz;
    HAL_I2C_Init(&I2cHandle);
}

static int codec_configure(void)
{
    uint16_t id = 0;
    if (codec_read(CODEC_ID_REG, &id) != 0 || id != CODEC_ID)
    {
        printf("Audio: NAU88C10 not found (id 0x%03x)\r\n", id);
        return -1;
    }
    /* Software reset; the codec may not acknowledge it. */
    (void)codec_write(0x00, 0x000);
    HAL_Delay(10);
    int failures = 0;
    for (uint32_t i = 0; i < sizeof(codec_init) / sizeof(codec_init[0]); i++)
    {
        if (codec_write((uint8_t)codec_init[i][0], codec_init[i][1]) != 0)
        {
            printf("Audio: codec register 0x%02x write failed\r\n", codec_init[i][0]);
            failures++;
        }
    }
    return failures == 0 ? 0 : -1;
}

static int codec_setup(void)
{
    uint32_t speed = I2cHandle.Init.ClockSpeed;
    i2c_speed(100000);
    int result = codec_configure();
    i2c_speed(speed);
    return result;
}

/* ---- I2S + DMA ----------------------------------------------------------- */

static void clocks_and_pins(void)
{
    GPIO_InitTypeDef gpio = {0};

    __HAL_RCC_GPIOB_CLK_ENABLE();
    __HAL_RCC_GPIOC_CLK_ENABLE();
    __HAL_RCC_SPI2_CLK_ENABLE();
    __HAL_RCC_DMA1_CLK_ENABLE();

    gpio.Mode      = GPIO_MODE_AF_PP;
    gpio.Pull      = GPIO_NOPULL;
    gpio.Speed     = GPIO_SPEED_FREQ_HIGH;
    gpio.Alternate = GPIO_AF5_SPI2;
    gpio.Pin       = GPIO_PIN_12 | GPIO_PIN_13 | GPIO_PIN_15;
    HAL_GPIO_Init(GPIOB, &gpio);
    gpio.Pin = GPIO_PIN_6;
    HAL_GPIO_Init(GPIOC, &gpio);

    /* PLLI2S from the main PLL input (HSE); SPI2 uses PLLI2S_R (DCKCFGR
     * I2S1SRC = 00 after reset). */
    RCC->CR &= ~RCC_CR_PLLI2SON;
    while (RCC->CR & RCC_CR_PLLI2SRDY)
    {
    }
    RCC->PLLI2SCFGR = (PLLI2S_M << RCC_PLLI2SCFGR_PLLI2SM_Pos) | (PLLI2S_N << RCC_PLLI2SCFGR_PLLI2SN_Pos) |
                      (PLLI2S_Q << RCC_PLLI2SCFGR_PLLI2SQ_Pos) | (PLLI2S_R << RCC_PLLI2SCFGR_PLLI2SR_Pos);
    RCC->DCKCFGR &= ~(3u << RCC_DCKCFGR_I2S1SRC_Pos);
    RCC->CR |= RCC_CR_PLLI2SON;
    while (!(RCC->CR & RCC_CR_PLLI2SRDY))
    {
    }
}

static void i2s_dma_start(void)
{
    memset(dma_buffer, 0, sizeof(dma_buffer));

    /* I2S2: master transmit, Philips, 16-bit data in 16-bit channels, MCLK. */
    SPI2->I2SCFGR = 0;
    SPI2->I2SPR   = SPI_I2SPR_MCKOE | (I2S_DIV << SPI_I2SPR_I2SDIV_Pos);
    SPI2->I2SCFGR = SPI_I2SCFGR_I2SMOD | (I2S_MASTER_TX << SPI_I2SCFGR_I2SCFG_Pos);
    SPI2->CR2     = SPI_CR2_TXDMAEN;

    /* DMA1 Stream 4, channel 0: memory -> SPI2_DR, 16 bit, circular, IRQ at
     * half and full transfer. */
    DMA1_Stream4->CR &= ~DMA_SxCR_EN;
    while (DMA1_Stream4->CR & DMA_SxCR_EN)
    {
    }
    DMA1->HIFCR        = DMA_HIFCR_CTCIF4 | DMA_HIFCR_CHTIF4 | DMA_HIFCR_CTEIF4 | DMA_HIFCR_CDMEIF4 | DMA_HIFCR_CFEIF4;
    DMA1_Stream4->PAR  = (uint32_t)&SPI2->DR;
    DMA1_Stream4->M0AR = (uint32_t)dma_buffer;
    DMA1_Stream4->NDTR = sizeof(dma_buffer) / sizeof(dma_buffer[0]);
    DMA1_Stream4->FCR  = 0; /* direct mode */
    DMA1_Stream4->CR   = (0u << DMA_SxCR_CHSEL_Pos) | DMA_SxCR_DIR_0 | DMA_SxCR_MINC | DMA_SxCR_PSIZE_0 |
                       DMA_SxCR_MSIZE_0 | DMA_SxCR_CIRC | (2u << DMA_SxCR_PL_Pos) | DMA_SxCR_HTIE | DMA_SxCR_TCIE;

    /* Does not call ThreadX services, so it may preempt anything below it. */
    NVIC_SetPriority(DMA1_Stream4_IRQn, 6);
    NVIC_EnableIRQ(DMA1_Stream4_IRQn);

    DMA1_Stream4->CR |= DMA_SxCR_EN;
    SPI2->I2SCFGR |= SPI_I2SCFGR_I2SE; /* starts MCLK, BCLK, WS */
}

/* ---- playback ------------------------------------------------------------ */

static void fill(uint32_t half)
{
    int16_t* frames = &dma_buffer[half * FRAMES_PER_HALF * 2u];
    uint32_t tail   = ring_tail;
    uint32_t head   = ring_head;
    __DMB(); /* samples before head are written */
    for (uint32_t i = 0; i < FRAMES_PER_HALF; i++)
    {
        int16_t sample = 0;
        if (tail != head)
        {
            sample = ring[tail & (RING_SIZE - 1u)];
            tail++;
        }
        frames[2 * i]     = sample;
        frames[2 * i + 1] = sample;
    }
    __DMB();
    ring_tail = tail;
}

void DMA1_Stream4_IRQHandler(void)
{
    uint32_t status = DMA1->HISR;
    if (status & DMA_HISR_HTIF4)
    {
        DMA1->HIFCR = DMA_HIFCR_CHTIF4;
        fill(0);
    }
    if (status & DMA_HISR_TCIF4)
    {
        DMA1->HIFCR = DMA_HIFCR_CTCIF4;
        fill(1);
    }
    if (status & (DMA_HISR_TEIF4 | DMA_HISR_DMEIF4 | DMA_HISR_FEIF4))
    {
        DMA1->HIFCR = DMA_HIFCR_CTEIF4 | DMA_HIFCR_CDMEIF4 | DMA_HIFCR_CFEIF4;
    }
}

/* ---- API ----------------------------------------------------------------- */

/* 0..100 % to DACGAIN: like the devkit SDK, 0.5 dB per percent from -50 dB
 * (1 %) to 0 dB (100 %); 0 = digital mute. */
static uint16_t volume_to_dacgain(uint32_t percent)
{
    if (percent > 100)
    {
        percent = 100;
    }
    return percent == 0 ? 0 : (uint16_t)(percent + 155);
}

void plat_audio_set_volume(uint32_t percent)
{
    if (!audio_ok)
    {
        return;
    }
    board_i2c_lock();
    uint32_t speed = I2cHandle.Init.ClockSpeed;
    i2c_speed(100000);
    if (codec_write(0x0B, volume_to_dacgain(percent)) != 0)
    {
        printf("Audio: volume write failed\r\n");
    }
    i2c_speed(speed);
    board_i2c_unlock();
}

void audio_init(void)
{
    clocks_and_pins();
    i2s_dma_start(); /* MCLK runs before the codec is configured */
    audio_ok = codec_setup() == 0;
    printf("Audio: %s\r\n", audio_ok ? "NAU88C10 ready (headphone jack, 8 kHz)" : "unavailable");
}

int32_t plat_audio_ok(void)
{
    return audio_ok;
}

uint32_t plat_audio_free(void)
{
    return RING_SIZE - (ring_head - ring_tail);
}

uint32_t plat_audio_pending(void)
{
    return ring_head - ring_tail;
}

uint32_t plat_audio_write(const int16_t* samples, uint32_t count)
{
    uint32_t head = ring_head;
    uint32_t free = RING_SIZE - (head - ring_tail);
    if (count > free)
    {
        count = free;
    }
    for (uint32_t i = 0; i < count; i++)
    {
        ring[(head + i) & (RING_SIZE - 1u)] = samples[i];
    }
    __DMB(); /* samples before the new head */
    ring_head = head + count;
    return count;
}

void plat_audio_clear(void)
{
    /* Only the producer calls this: drop everything not yet played. */
    NVIC_DisableIRQ(DMA1_Stream4_IRQn);
    ring_tail = ring_head;
    NVIC_EnableIRQ(DMA1_Stream4_IRQn);
}

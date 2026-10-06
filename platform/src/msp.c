/*
 * SPDX-License-Identifier: MIT
 *
 * HAL MSP callbacks (pin muxing). Derived from stm32f4xx_hal_msp.c of the
 * Azure RTOS getting-started guides, Copyright (c) Microsoft Corporation.
 * Button EXTI interrupts are not enabled: buttons are polled.
 */

#include "stm32f4xx_hal.h"

void HAL_MspInit(void)
{
}

/* I2C1: SCL PB8, SDA PB9 (sensors, OLED) */
void HAL_I2C_MspInit(I2C_HandleTypeDef* hi2c)
{
    (void)hi2c;
    GPIO_InitTypeDef gpio = {0};

    __HAL_RCC_GPIOB_CLK_ENABLE();
    __HAL_RCC_I2C1_CLK_ENABLE();

    gpio.Pin       = GPIO_PIN_8 | GPIO_PIN_9;
    gpio.Mode      = GPIO_MODE_AF_OD;
    gpio.Pull      = GPIO_PULLUP;
    gpio.Speed     = GPIO_SPEED_HIGH;
    gpio.Alternate = GPIO_AF4_I2C1;
    HAL_GPIO_Init(GPIOB, &gpio);
}

/* USART6: TX PA11, RX PA12 (console via the on-board debug probe) */
void HAL_UART_MspInit(UART_HandleTypeDef* huart)
{
    if (huart->Instance != USART6)
    {
        return;
    }
    GPIO_InitTypeDef gpio = {0};

    __HAL_RCC_USART6_CLK_ENABLE();
    __HAL_RCC_GPIOA_CLK_ENABLE();

    gpio.Pin       = GPIO_PIN_11 | GPIO_PIN_12;
    gpio.Mode      = GPIO_MODE_AF_PP;
    gpio.Pull      = GPIO_NOPULL;
    gpio.Speed     = GPIO_SPEED_FREQ_LOW;
    gpio.Alternate = GPIO_AF8_USART6;
    HAL_GPIO_Init(GPIOA, &gpio);
}

/*
 * SPDX-License-Identifier: MIT
 *
 * MXCHIP AZ3166 pins. Derived from board_init.h of the Azure RTOS
 * getting-started guides, Copyright (c) Microsoft Corporation.
 */

#ifndef AZ3166_BOARD_H
#define AZ3166_BOARD_H

#include "stm32f4xx_hal.h"

#define BUTTON_A_PIN GPIO_PIN_4
#define BUTTON_B_PIN GPIO_PIN_10

#define BUTTON_A_IS_PRESSED ((GPIOA->IDR & BUTTON_A_PIN) == 0)
#define BUTTON_B_IS_PRESSED ((GPIOA->IDR & BUTTON_B_PIN) == 0)

#define WIFI_LED_ON()  GPIOB->BSRR = GPIO_PIN_2
#define WIFI_LED_OFF() GPIOB->BSRR = (uint32_t)GPIO_PIN_2 << 16

#define AZURE_LED_ON()  GPIOA->BSRR = GPIO_PIN_15
#define AZURE_LED_OFF() GPIOA->BSRR = (uint32_t)GPIO_PIN_15 << 16

#define USER_LED_ON()  GPIOC->BSRR = GPIO_PIN_13
#define USER_LED_OFF() GPIOC->BSRR = (uint32_t)GPIO_PIN_13 << 16

#define RGB_LED_SET_R(value) TIM3->CCR1 = (value)
#define RGB_LED_SET_G(value) TIM2->CCR2 = (value)
#define RGB_LED_SET_B(value) TIM3->CCR2 = (value)

extern UART_HandleTypeDef UartHandle;
extern I2C_HandleTypeDef I2cHandle;

/* Hardware init before the kernel starts (no RTOS services). */
void board_init(void);
/* Creates the RTOS objects used by the board functions. */
void board_rtos_init(void);

#endif

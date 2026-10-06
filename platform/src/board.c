/*
 * SPDX-License-Identifier: MIT
 *
 * MXCHIP AZ3166 board support: clocks, console, LEDs, buttons, sensors, OLED,
 * RNG, unique ID. Clock, GPIO, timer, I2C and UART setup derived from
 * board_init.c of the Azure RTOS getting-started guides,
 * Copyright (c) Microsoft Corporation.
 */

#include "board.h"

#include <stdio.h>
#include <string.h>

#include "sensor.h"
#include "ssd1306.h"
#include "ssd1306_fonts.h"
#include "tx_api.h"

#include "platform.h"

I2C_HandleTypeDef I2cHandle;   /* name used by the BSP sensor and OLED drivers */
UART_HandleTypeDef UartHandle;

#define I2C_ADDRESS    0x30F
#define I2C_SPEEDCLOCK 400000

#define RGB_PERIOD 2047 /* timer auto-reload: PWM resolution */

#define DISPLAY_LINES 4
#define DISPLAY_CHARS 18 /* 128 px / 7 px font */

static TX_MUTEX i2c_mutex;
static TX_MUTEX log_mutex;
static int rtos_ready;

static int sensor_status[4];
static uint8_t rgb_state[3];
static char display[DISPLAY_LINES][DISPLAY_CHARS + 1];
static int display_ok;

static void SystemClock_Config(void);
static void Error_Handler(void);

/* ---- HAL time base -------------------------------------------------------
 * ThreadX owns SysTick, so the HAL tick is derived from the DWT cycle
 * counter. It works before the kernel starts and needs no interrupt. The
 * 32-bit counter wraps every ~44 s at 96 MHz; it is accumulated into 64 bits,
 * which requires a call at least that often (the UDS worker ticks every
 * 100 ms). */

static uint64_t cycles_accumulated;
static uint32_t cycles_last;

static uint64_t cycles_now(void)
{
    uint32_t primask = __get_PRIMASK();
    __disable_irq();
    uint32_t now = DWT->CYCCNT;
    cycles_accumulated += (uint32_t)(now - cycles_last);
    cycles_last = now;
    uint64_t total = cycles_accumulated;
    __set_PRIMASK(primask);
    return total;
}

HAL_StatusTypeDef HAL_InitTick(uint32_t TickPriority)
{
    (void)TickPriority;
    return HAL_OK;
}

uint32_t HAL_GetTick(void)
{
    return (uint32_t)(cycles_now() / (SystemCoreClock / 1000u));
}

uint64_t plat_uptime_us(void)
{
    return cycles_now() / (SystemCoreClock / 1000000u);
}

static void cycle_counter_init(void)
{
    CoreDebug->DEMCR |= CoreDebug_DEMCR_TRCENA_Msk;
    DWT->CYCCNT = 0;
    DWT->CTRL |= DWT_CTRL_CYCCNTENA_Msk;
}

/* ---- init ---------------------------------------------------------------- */

static void UART_Console_Init(void)
{
    UartHandle.Instance          = USART6;
    UartHandle.Init.BaudRate     = 115200;
    UartHandle.Init.WordLength   = UART_WORDLENGTH_8B;
    UartHandle.Init.StopBits     = UART_STOPBITS_1;
    UartHandle.Init.Parity       = UART_PARITY_NONE;
    UartHandle.Init.HwFlowCtl    = UART_HWCONTROL_NONE;
    UartHandle.Init.Mode         = UART_MODE_TX_RX;
    UartHandle.Init.OverSampling = UART_OVERSAMPLING_16;
    if (HAL_UART_Init(&UartHandle) != HAL_OK)
    {
        Error_Handler();
    }
}

static void TIM_Init(void)
{
    TIM_HandleTypeDef timer_handle = {0};
    TIM_OC_InitTypeDef channel_config = {0};

    __HAL_RCC_TIM2_CLK_ENABLE();
    __HAL_RCC_TIM3_CLK_ENABLE();

    timer_handle.Instance               = TIM2;
    timer_handle.Init.Prescaler         = 45;
    timer_handle.Init.Period            = RGB_PERIOD;
    timer_handle.Init.ClockDivision     = 0;
    timer_handle.Init.CounterMode       = TIM_COUNTERMODE_UP;
    timer_handle.Init.RepetitionCounter = 0;
    timer_handle.Init.AutoReloadPreload = TIM_AUTORELOAD_PRELOAD_DISABLE;
    HAL_TIM_PWM_Init(&timer_handle);

    channel_config.OCMode       = TIM_OCMODE_PWM1;
    channel_config.OCPolarity   = TIM_OCPOLARITY_HIGH;
    channel_config.OCFastMode   = TIM_OCFAST_DISABLE;
    channel_config.OCNPolarity  = TIM_OCNPOLARITY_HIGH;
    channel_config.OCNIdleState = TIM_OCNIDLESTATE_RESET;
    channel_config.OCIdleState  = TIM_OCIDLESTATE_RESET;
    channel_config.Pulse        = 0;

    /* green */
    HAL_TIM_PWM_ConfigChannel(&timer_handle, &channel_config, TIM_CHANNEL_2);
    HAL_TIM_PWM_Start(&timer_handle, TIM_CHANNEL_2);

    /* red (CH1), blue (CH2) */
    timer_handle.Instance = TIM3;
    HAL_TIM_PWM_Init(&timer_handle);
    HAL_TIM_PWM_ConfigChannel(&timer_handle, &channel_config, TIM_CHANNEL_1);
    HAL_TIM_PWM_Start(&timer_handle, TIM_CHANNEL_1);
    HAL_TIM_PWM_ConfigChannel(&timer_handle, &channel_config, TIM_CHANNEL_2);
    HAL_TIM_PWM_Start(&timer_handle, TIM_CHANNEL_2);
}

static void GPIO_Init(void)
{
    GPIO_InitTypeDef gpio = {0};

    __HAL_RCC_GPIOA_CLK_ENABLE();
    __HAL_RCC_GPIOB_CLK_ENABLE();
    __HAL_RCC_GPIOC_CLK_ENABLE();

    /* LEDs: Wi-Fi PB2, Azure PA15, user PC13 */
    gpio.Mode  = GPIO_MODE_OUTPUT_PP;
    gpio.Speed = GPIO_SPEED_FREQ_LOW;
    gpio.Pull  = GPIO_NOPULL;
    gpio.Pin   = GPIO_PIN_2;
    HAL_GPIO_Init(GPIOB, &gpio);
    gpio.Pin = GPIO_PIN_15;
    HAL_GPIO_Init(GPIOA, &gpio);
    gpio.Pin = GPIO_PIN_13;
    HAL_GPIO_Init(GPIOC, &gpio);

    /* Buttons A (PA4), B (PA10): polled, active low */
    gpio.Pin  = BUTTON_A_PIN | BUTTON_B_PIN;
    gpio.Mode = GPIO_MODE_INPUT;
    HAL_GPIO_Init(GPIOA, &gpio);

    /* RGB LED: red PB4 (TIM3_CH1), blue PC7 (TIM3_CH2), green PB3 (TIM2_CH2) */
    gpio.Mode      = GPIO_MODE_AF_PP;
    gpio.Pin       = GPIO_PIN_4;
    gpio.Alternate = GPIO_AF2_TIM3;
    HAL_GPIO_Init(GPIOB, &gpio);
    gpio.Pin = GPIO_PIN_7;
    HAL_GPIO_Init(GPIOC, &gpio);
    gpio.Pin       = GPIO_PIN_3;
    gpio.Alternate = GPIO_AF1_TIM2;
    HAL_GPIO_Init(GPIOB, &gpio);
}

static void I2C1_Init(void)
{
    I2cHandle.Instance             = I2C1;
    I2cHandle.Init.ClockSpeed      = I2C_SPEEDCLOCK;
    I2cHandle.Init.DutyCycle       = I2C_DUTYCYCLE_2;
    I2cHandle.Init.OwnAddress1     = I2C_ADDRESS;
    I2cHandle.Init.AddressingMode  = I2C_ADDRESSINGMODE_10BIT;
    I2cHandle.Init.DualAddressMode = I2C_DUALADDRESS_DISABLE;
    I2cHandle.Init.OwnAddress2     = 0xFF;
    I2cHandle.Init.GeneralCallMode = I2C_GENERALCALL_DISABLE;
    I2cHandle.Init.NoStretchMode   = I2C_NOSTRETCH_DISABLE;
    if (HAL_I2C_Init(&I2cHandle) != HAL_OK)
    {
        Error_Handler();
    }
}

static void RNG_Init(void)
{
    /* RNG runs from the 48 MHz PLL Q output configured in SystemClock_Config. */
    __HAL_RCC_RNG_CLK_ENABLE();
    RNG->CR |= RNG_CR_RNGEN;
}

static void Sensors_Init(void)
{
    sensor_status[PLAT_SENSOR_HTS221]  = hts221_config() == SENSOR_OK;
    sensor_status[PLAT_SENSOR_LPS22HB] = lps22hb_config() == SENSOR_OK;
    sensor_status[PLAT_SENSOR_LSM6DSL] = lsm6dsl_config() == SENSOR_OK;
    sensor_status[PLAT_SENSOR_LIS2MDL] = lis2mdl_config() == SENSOR_OK;

    printf("Sensors: HTS221 %s, LPS22HB %s, LSM6DSL %s, LIS2MDL %s\r\n",
        sensor_status[0] ? "ok" : "FAIL",
        sensor_status[1] ? "ok" : "FAIL",
        sensor_status[2] ? "ok" : "FAIL",
        sensor_status[3] ? "ok" : "FAIL");
}

static void Display_Init(void)
{
    display_ok = HAL_I2C_IsDeviceReady(&I2cHandle, SSD1306_I2C_ADDR, 3, 10) == HAL_OK;
    if (!display_ok)
    {
        printf("OLED not found\r\n");
        return;
    }
    for (int i = 0; i < DISPLAY_LINES; i++)
    {
        memset(display[i], ' ', DISPLAY_CHARS);
    }
    ssd1306_Init();
}

void board_init(void)
{
    HAL_Init();
    SystemClock_Config();
    cycle_counter_init();

    UART_Console_Init();
    TIM_Init();
    GPIO_Init();
    I2C1_Init();
    RNG_Init();

    printf("\r\nFLXC1000 AZ3166 starting\r\n");

    Sensors_Init();
    Display_Init();
}

void board_rtos_init(void)
{
    tx_mutex_create(&i2c_mutex, "i2c", TX_INHERIT);
    tx_mutex_create(&log_mutex, "log", TX_INHERIT);
    rtos_ready = 1;
}

/**
 * System clock: HSE 26 MHz -> PLL -> 96 MHz SYSCLK, 48 MHz PLL Q (RNG).
 * APB1 48 MHz, APB2 96 MHz, 3 flash wait states.
 */
static void SystemClock_Config(void)
{
    RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};
    RCC_OscInitTypeDef RCC_OscInitStruct = {0};

    __HAL_RCC_PWR_CLK_ENABLE();
    __HAL_PWR_VOLTAGESCALING_CONFIG(PWR_REGULATOR_VOLTAGE_SCALE1);

    RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_HSE | RCC_OSCILLATORTYPE_LSE;
    RCC_OscInitStruct.HSEState       = RCC_HSE_ON;
    RCC_OscInitStruct.LSEState       = RCC_LSE_ON;
    RCC_OscInitStruct.PLL.PLLState   = RCC_PLL_ON;
    RCC_OscInitStruct.PLL.PLLSource  = RCC_PLLSOURCE_HSE;
    RCC_OscInitStruct.PLL.PLLM       = 13;
    RCC_OscInitStruct.PLL.PLLN       = 96;
    RCC_OscInitStruct.PLL.PLLP       = RCC_PLLP_DIV2;
    RCC_OscInitStruct.PLL.PLLQ       = 4;
    RCC_OscInitStruct.PLL.PLLR       = 2;
    if (HAL_RCC_OscConfig(&RCC_OscInitStruct) != HAL_OK)
    {
        Error_Handler();
    }

    RCC_ClkInitStruct.ClockType =
        RCC_CLOCKTYPE_SYSCLK | RCC_CLOCKTYPE_HCLK | RCC_CLOCKTYPE_PCLK1 | RCC_CLOCKTYPE_PCLK2;
    RCC_ClkInitStruct.SYSCLKSource   = RCC_SYSCLKSOURCE_PLLCLK;
    RCC_ClkInitStruct.AHBCLKDivider  = RCC_SYSCLK_DIV1;
    RCC_ClkInitStruct.APB1CLKDivider = RCC_HCLK_DIV2;
    RCC_ClkInitStruct.APB2CLKDivider = RCC_HCLK_DIV1;
    if (HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_3) != HAL_OK)
    {
        Error_Handler();
    }
}

static void Error_Handler(void)
{
    printf("FATAL: STM32 error handler\r\n");
    while (1)
    {
    }
}

/* ---- locking ------------------------------------------------------------- */

static void i2c_lock(void)
{
    if (rtos_ready)
    {
        tx_mutex_get(&i2c_mutex, TX_WAIT_FOREVER);
    }
}

static void i2c_unlock(void)
{
    if (rtos_ready)
    {
        tx_mutex_put(&i2c_mutex);
    }
}

/* ---- platform API -------------------------------------------------------- */

void plat_log(const uint8_t* msg, uint32_t len)
{
    if (rtos_ready)
    {
        tx_mutex_get(&log_mutex, TX_WAIT_FOREVER);
    }
    HAL_UART_Transmit(&UartHandle, (uint8_t*)msg, len, HAL_MAX_DELAY);
    if (rtos_ready)
    {
        tx_mutex_put(&log_mutex);
    }
}

void plat_sleep_ms(uint32_t ms)
{
    ULONG ticks = (ms * TX_TIMER_TICKS_PER_SECOND + 999u) / 1000u;
    tx_thread_sleep(ticks ? ticks : 1);
}

uint32_t plat_random(void)
{
    for (int tries = 0; tries < 1000; tries++)
    {
        uint32_t sr = RNG->SR;
        if (sr & (RNG_SR_CECS | RNG_SR_SECS))
        {
            /* Clock or seed error: restart the generator. */
            RNG->CR &= ~RNG_CR_RNGEN;
            RNG->SR = 0;
            RNG->CR |= RNG_CR_RNGEN;
        }
        else if (sr & RNG_SR_DRDY)
        {
            return RNG->DR;
        }
    }
    return DWT->CYCCNT * 2654435761u; /* fallback: Knuth hash of the cycle counter */
}

void plat_reset(void)
{
    printf("Resetting\r\n");
    NVIC_SystemReset();
    while (1)
    {
    }
}

void plat_led_set(uint32_t index, uint32_t duty_percent)
{
    uint32_t duty = duty_percent > 100 ? 100 : duty_percent;
    uint32_t pwm  = duty * RGB_PERIOD / 100u;
    switch (index)
    {
        case 0:
            if (duty)
            {
                USER_LED_ON();
            }
            else
            {
                USER_LED_OFF();
            }
            break;
        case 1:
            if (duty)
            {
                AZURE_LED_ON();
            }
            else
            {
                AZURE_LED_OFF();
            }
            break;
        case 2:
            RGB_LED_SET_R(pwm);
            break;
        case 3:
            RGB_LED_SET_G(pwm);
            break;
        case 4:
            RGB_LED_SET_B(pwm);
            break;
        default:
            break;
    }
}

void plat_rgb_set(uint8_t r, uint8_t g, uint8_t b)
{
    rgb_state[0] = r;
    rgb_state[1] = g;
    rgb_state[2] = b;
    RGB_LED_SET_R((uint32_t)r * RGB_PERIOD / 255u);
    RGB_LED_SET_G((uint32_t)g * RGB_PERIOD / 255u);
    RGB_LED_SET_B((uint32_t)b * RGB_PERIOD / 255u);
}

void plat_display_line(uint32_t line, const uint8_t* text, uint32_t len)
{
    if (!display_ok || line >= DISPLAY_LINES)
    {
        return;
    }
    i2c_lock();
    memset(display[line], ' ', DISPLAY_CHARS);
    for (uint32_t i = 0; i < len && i < DISPLAY_CHARS; i++)
    {
        char c         = (char)text[i];
        display[line][i] = (c >= 0x20 && c < 0x7F) ? c : ' ';
    }
    display[line][DISPLAY_CHARS] = '\0';

    ssd1306_Fill(Black);
    for (int i = 0; i < DISPLAY_LINES; i++)
    {
        ssd1306_SetCursor(2, (uint8_t)(i * 16));
        ssd1306_WriteString(display[i], Font_7x10, White);
    }
    ssd1306_UpdateScreen();
    i2c_unlock();
}

uint32_t plat_buttons(void)
{
    return (BUTTON_A_IS_PRESSED ? 1u : 0u) | (BUTTON_B_IS_PRESSED ? 2u : 0u);
}

void plat_serial_number(uint8_t out[12])
{
    memcpy(out, (const void*)UID_BASE, 12);
}

int32_t plat_sensor_ok(uint32_t sensor)
{
    return sensor < 4 ? sensor_status[sensor] : 0;
}

int32_t plat_read_hts221(float* temperature_c, float* humidity_pct)
{
    if (!sensor_status[PLAT_SENSOR_HTS221])
    {
        return -1;
    }
    i2c_lock();
    hts221_data_t data = hts221_data_read();
    i2c_unlock();
    *temperature_c = data.temperature_degC;
    *humidity_pct  = data.humidity_perc;
    return 0;
}

int32_t plat_read_lps22hb(float* pressure_hpa)
{
    if (!sensor_status[PLAT_SENSOR_LPS22HB])
    {
        return -1;
    }
    i2c_lock();
    lps22hb_t data = lps22hb_data_read();
    i2c_unlock();
    *pressure_hpa = data.pressure_hPa;
    return 0;
}

int32_t plat_read_lsm6dsl(float acceleration_mg[3], float angular_rate_mdps[3])
{
    if (!sensor_status[PLAT_SENSOR_LSM6DSL])
    {
        return -1;
    }
    i2c_lock();
    lsm6dsl_data_t data = lsm6dsl_data_read();
    i2c_unlock();
    memcpy(acceleration_mg, data.acceleration_mg, sizeof(data.acceleration_mg));
    memcpy(angular_rate_mdps, data.angular_rate_mdps, sizeof(data.angular_rate_mdps));
    return 0;
}

int32_t plat_read_lis2mdl(float magnetic_mg[3])
{
    if (!sensor_status[PLAT_SENSOR_LIS2MDL])
    {
        return -1;
    }
    i2c_lock();
    lis2mdl_data_t data = lis2mdl_data_read();
    i2c_unlock();
    memcpy(magnetic_mg, data.magnetic_mG, sizeof(data.magnetic_mG));
    return 0;
}

/* ---- newlib console ------------------------------------------------------ */

int _write(int file, char* ptr, int len)
{
    (void)file;
    plat_log((const uint8_t*)ptr, (uint32_t)len);
    return len;
}

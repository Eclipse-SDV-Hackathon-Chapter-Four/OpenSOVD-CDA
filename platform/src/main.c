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
 * AZ3166 ECU: startup, threads and the UDS worker. Built twice:
 *
 *   FW_IMAGE_BOOT  bootloader at 0x08000000: picks and starts an app slot,
 *                  otherwise runs the Boot variant (DoIP programming)
 *   FW_IMAGE_APP   app linked for slot FW_SLOT ('A' or 'B'): App variant;
 *                  confirms its slot once DoIP is up
 *
 * Threads (priority, stack):
 *   uds      (4, 28 KiB) ECU init, network bring-up, then runs every UDS
 *                        request (the only thread touching the ECU)
 *   tcp0/1   (6,  3 KiB) DoIP TCP connections (az3166_tcp_task)
 *   udp      (6,  2 KiB) DoIP vehicle identification (az3166_udp_task)
 *   routine  (8,  2 KiB) LED self-test, display (az3166_routine_task)
 *   speech   (9,  4 KiB) App only: text-to-speech into the audio output
 *                        (az3166_speech_task)
 *   presence (7,  3 KiB) App only: occupancy + temperature alarm, once a
 *                        second (az3166_presence_task)
 */

#include <stdio.h>

#include "tx_api.h"

#include "audio.h"
#include "board.h"
#include "boot_state.h"
#include "bootloader.h"
#include "image_info.h"
#include "net.h"
#include "platform.h"

void syscalls_rtos_init(void);

#if !defined(FW_IMAGE_BOOT) && !defined(FW_IMAGE_APP)
#error "define FW_IMAGE_BOOT or FW_IMAGE_APP"
#endif
#ifndef FW_VERSION
#define FW_VERSION "0.0.0"
#endif

#ifndef WIFI_SSID
#define WIFI_SSID ""
#endif
#ifndef WIFI_PASSWORD
#define WIFI_PASSWORD ""
#endif

#define UDS_STACK_SIZE     (28 * 1024)
#define TCP_STACK_SIZE     (3 * 1024)
#define UDP_STACK_SIZE     (2 * 1024)
#define ROUTINE_STACK_SIZE (2 * 1024)
#define SPEECH_STACK_SIZE  (4 * 1024)
#define PRESENCE_STACK_SIZE (3 * 1024)

#define UDS_PRIORITY     4
#define NET_PRIORITY     6
#define ROUTINE_PRIORITY 8
#define SPEECH_PRIORITY  9
#define PRESENCE_PRIORITY 7

#define UDS_TICK_MS 100

/* An app on trial must come up (DoIP listening) within this time, else it
 * resets and the bootloader counts a failed attempt. */
#define TRIAL_DEADLINE_S 120

extern uint32_t g_pfnVectors[];

static TX_THREAD uds_thread;
static TX_THREAD tcp_threads[PLAT_TCP_SLOTS];
static TX_THREAD udp_thread;
static TX_THREAD routine_thread;
#ifdef FW_IMAGE_APP
static TX_THREAD speech_thread;
static ULONG speech_stack[SPEECH_STACK_SIZE / sizeof(ULONG)];
static TX_THREAD presence_thread;
static ULONG presence_stack[PRESENCE_STACK_SIZE / sizeof(ULONG)];
#endif

static ULONG uds_stack[UDS_STACK_SIZE / sizeof(ULONG)];
static ULONG tcp_stacks[PLAT_TCP_SLOTS][TCP_STACK_SIZE / sizeof(ULONG)];
static ULONG udp_stack[UDP_STACK_SIZE / sizeof(ULONG)];
static ULONG routine_stack[ROUTINE_STACK_SIZE / sizeof(ULONG)];

static uint32_t boot_state;

#ifdef FW_IMAGE_APP
/* Checked by the bootloader before it starts the slot. */
__attribute__((section(".image_info"), used)) static const image_info_t image_info = {
    .magic     = IMAGE_INFO_MAGIC,
    .link_base = (uint32_t)g_pfnVectors,
    .version   = FW_VERSION,
};

static TX_TIMER trial_timer;
static volatile int confirmed;

static uint8_t own_slot(void)
{
    return slot_of_address((uint32_t)g_pfnVectors);
}

static int on_trial(void)
{
    boot_state_t state;
    boot_state_get(&state);
    return state.trial == own_slot();
}

static void trial_deadline(ULONG parameter)
{
    (void)parameter;
    if (!confirmed)
    {
        printf("Trial: not up within %d s, resetting\r\n", TRIAL_DEADLINE_S);
        NVIC_SystemReset();
    }
}

/* The app is up: keep this slot. */
static void confirm_slot(void)
{
    if (on_trial())
    {
        if (boot_state_append(REC_CONFIRMED, own_slot()) == 0)
        {
            printf("Trial: slot %c confirmed\r\n", 'A' + own_slot());
        }
    }
    confirmed = 1;
}
#endif

void plat_version(uint8_t out[16])
{
    static const char version[16] = FW_VERSION;
    for (int i = 0; i < 16; i++)
    {
        out[i] = (uint8_t)version[i];
    }
}

uint8_t plat_running_slot(void)
{
#ifdef FW_IMAGE_APP
    return (uint8_t)('A' + own_slot());
#else
    return 'L';
#endif
}

/* ---- UDS worker ------------------------------------------------------------
 * DoIP threads post a pointer to a request on stack and wait on its
 * semaphore; the worker executes it and signals completion. */

typedef struct
{
    uint16_t source;
    const uint8_t* req;
    uint32_t len;
    uint8_t* resp;
    uint32_t cap;
    uint32_t result;
    TX_SEMAPHORE done;
} uds_job_t;

static TX_QUEUE uds_queue;
static ULONG uds_queue_area[PLAT_TCP_SLOTS];

uint32_t plat_uds_execute(uint16_t source, const uint8_t* req, uint32_t len, uint8_t* resp, uint32_t cap)
{
    uds_job_t job = {.source = source, .req = req, .len = len, .resp = resp, .cap = cap};
    uds_job_t* job_ptr = &job;

    tx_semaphore_create(&job.done, "uds-job", 0);
    tx_queue_send(&uds_queue, &job_ptr, TX_WAIT_FOREVER);
    tx_semaphore_get(&job.done, TX_WAIT_FOREVER);
    tx_semaphore_delete(&job.done);
    return job.result;
}

static void uds_thread_entry(ULONG parameter)
{
    (void)parameter;

    az3166_init(boot_state);
    tx_thread_resume(&routine_thread);

    if (net_init(WIFI_SSID, WIFI_PASSWORD) == 0)
    {
        for (uint32_t i = 0; i < PLAT_TCP_SLOTS; i++)
        {
            tx_thread_resume(&tcp_threads[i]);
        }
        tx_thread_resume(&udp_thread);
#ifdef FW_IMAGE_APP
        confirm_slot();
        tx_thread_resume(&presence_thread);
#endif
    }

    for (;;)
    {
        uds_job_t* job;
        if (tx_queue_receive(&uds_queue, &job, UDS_TICK_MS * TX_TIMER_TICKS_PER_SECOND / 1000) == TX_SUCCESS)
        {
            job->result = az3166_uds_execute(job->source, job->req, job->len, job->resp, job->cap);
            tx_semaphore_put(&job->done);
        }
        az3166_uds_tick();
    }
}

static void tcp_thread_entry(ULONG slot)
{
    az3166_tcp_task(slot);
}

static void udp_thread_entry(ULONG parameter)
{
    (void)parameter;
    az3166_udp_task();
}

static void routine_thread_entry(ULONG parameter)
{
    (void)parameter;
    az3166_routine_task();
}

#ifdef FW_IMAGE_APP
static void speech_thread_entry(ULONG parameter)
{
    (void)parameter;
    az3166_speech_task();
}

static void presence_thread_entry(ULONG parameter)
{
    (void)parameter;
    az3166_presence_task();
}
#endif

void tx_application_define(void* first_unused_memory)
{
    (void)first_unused_memory;

    syscalls_rtos_init();
    board_rtos_init();
    tx_queue_create(&uds_queue, "uds", TX_1_ULONG, uds_queue_area, sizeof(uds_queue_area));

    tx_thread_create(&uds_thread, "uds", uds_thread_entry, 0, uds_stack, sizeof(uds_stack), UDS_PRIORITY,
        UDS_PRIORITY, TX_NO_TIME_SLICE, TX_AUTO_START);

    /* Started by the UDS worker once the ECU and network are up. */
    for (ULONG i = 0; i < PLAT_TCP_SLOTS; i++)
    {
        tx_thread_create(&tcp_threads[i], "tcp", tcp_thread_entry, i, tcp_stacks[i], sizeof(tcp_stacks[i]),
            NET_PRIORITY, NET_PRIORITY, TX_NO_TIME_SLICE, TX_DONT_START);
    }
    tx_thread_create(&udp_thread, "udp", udp_thread_entry, 0, udp_stack, sizeof(udp_stack), NET_PRIORITY,
        NET_PRIORITY, TX_NO_TIME_SLICE, TX_DONT_START);
    tx_thread_create(&routine_thread, "routine", routine_thread_entry, 0, routine_stack, sizeof(routine_stack),
        ROUTINE_PRIORITY, ROUTINE_PRIORITY, TX_NO_TIME_SLICE, TX_DONT_START);

#ifdef FW_IMAGE_APP
    /* Lowest priority: renders in the idle time, the watchdog kick in the
     * routine thread stays above it. */
    tx_thread_create(&speech_thread, "speech", speech_thread_entry, 0, speech_stack, sizeof(speech_stack),
        SPEECH_PRIORITY, SPEECH_PRIORITY, TX_NO_TIME_SLICE, TX_AUTO_START);
    /* Started by the UDS worker once the network is up. */
    tx_thread_create(&presence_thread, "presence", presence_thread_entry, 0, presence_stack,
        sizeof(presence_stack), PRESENCE_PRIORITY, PRESENCE_PRIORITY, TX_NO_TIME_SLICE, TX_DONT_START);

    if (on_trial())
    {
        printf("Trial: slot %c must come up within %d s\r\n", 'A' + own_slot(), TRIAL_DEADLINE_S);
        tx_timer_create(&trial_timer, "trial", trial_deadline, 0, TRIAL_DEADLINE_S * TX_TIMER_TICKS_PER_SECOND, 0,
            TX_AUTO_ACTIVATE);
    }
#endif
}

#ifdef FW_IMAGE_BOOT
int main(void)
{
    /* Before any clock setup: a started app must begin from reset state. */
    bootloader_select();

    board_init();
    printf("Bootloader %s: Boot variant\r\n", FW_VERSION);
    boot_state = PLAT_BOOT_STATE_BOOT_REQUESTED;
    tx_kernel_enter();
    return 0;
}
#else
int main(void)
{
    /* SystemInit pointed the vector table at the bootloader. */
    SCB->VTOR = (uint32_t)g_pfnVectors;

    board_init();
    printf("App %s in slot %c\r\n", FW_VERSION, plat_running_slot());
    audio_init();
#ifdef APP_TEST_HANG
    /* Test build: a broken app that never comes up (rollback test). */
    printf("APP_TEST_HANG: hanging\r\n");
    for (;;)
    {
    }
#endif
    boot_state = PLAT_BOOT_STATE_APP_VALID;
    tx_kernel_enter();
    return 0;
}
#endif

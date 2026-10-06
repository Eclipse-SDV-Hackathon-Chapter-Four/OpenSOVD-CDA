/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 *
 * FLXC1000 on the MXCHIP AZ3166: startup, threads and the UDS worker.
 *
 * Threads (priority, stack):
 *   uds      (4, 20 KiB) ECU init, network bring-up, then runs every UDS
 *                        request (the only thread touching the ECU)
 *   tcp0/1   (6,  3 KiB) DoIP TCP connections (flxc1000_tcp_task)
 *   udp      (6,  2 KiB) DoIP vehicle identification (flxc1000_udp_task)
 *   routine  (8,  2 KiB) LED self-test, display (flxc1000_routine_task)
 */

#include <stdio.h>

#include "tx_api.h"

#include "board.h"
#include "boot_state.h"
#include "net.h"
#include "platform.h"

void syscalls_rtos_init(void);

#ifndef WIFI_SSID
#define WIFI_SSID ""
#endif
#ifndef WIFI_PASSWORD
#define WIFI_PASSWORD ""
#endif

#define UDS_STACK_SIZE     (20 * 1024)
#define TCP_STACK_SIZE     (3 * 1024)
#define UDP_STACK_SIZE     (2 * 1024)
#define ROUTINE_STACK_SIZE (2 * 1024)

#define UDS_PRIORITY     4
#define NET_PRIORITY     6
#define ROUTINE_PRIORITY 8

#define UDS_TICK_MS 100

static TX_THREAD uds_thread;
static TX_THREAD tcp_threads[PLAT_TCP_SLOTS];
static TX_THREAD udp_thread;
static TX_THREAD routine_thread;

static ULONG uds_stack[UDS_STACK_SIZE / sizeof(ULONG)];
static ULONG tcp_stacks[PLAT_TCP_SLOTS][TCP_STACK_SIZE / sizeof(ULONG)];
static ULONG udp_stack[UDP_STACK_SIZE / sizeof(ULONG)];
static ULONG routine_stack[ROUTINE_STACK_SIZE / sizeof(ULONG)];

static uint32_t boot_state;

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

    flxc1000_init(boot_state);
    tx_thread_resume(&routine_thread);

    if (net_init(WIFI_SSID, WIFI_PASSWORD) == 0)
    {
        for (uint32_t i = 0; i < PLAT_TCP_SLOTS; i++)
        {
            tx_thread_resume(&tcp_threads[i]);
        }
        tx_thread_resume(&udp_thread);
    }

    for (;;)
    {
        uds_job_t* job;
        if (tx_queue_receive(&uds_queue, &job, UDS_TICK_MS * TX_TIMER_TICKS_PER_SECOND / 1000) == TX_SUCCESS)
        {
            job->result = flxc1000_uds_execute(job->source, job->req, job->len, job->resp, job->cap);
            tx_semaphore_put(&job->done);
        }
        flxc1000_uds_tick();
    }
}

static void tcp_thread_entry(ULONG slot)
{
    flxc1000_tcp_task(slot);
}

static void udp_thread_entry(ULONG parameter)
{
    (void)parameter;
    flxc1000_udp_task();
}

static void routine_thread_entry(ULONG parameter)
{
    (void)parameter;
    flxc1000_routine_task();
}

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
}

/* Boot variant if requested by the App (ECUReset) or if button B is held
 * during reset; otherwise the App (also when no state was ever stored). */
static uint32_t select_boot_state(void)
{
    if (BUTTON_B_IS_PRESSED)
    {
        printf("Button B held: starting Boot variant\r\n");
        return PLAT_BOOT_STATE_BOOT_REQUESTED;
    }
    if (boot_state_read() == PLAT_BOOT_STATE_BOOT_REQUESTED)
    {
        return PLAT_BOOT_STATE_BOOT_REQUESTED;
    }
    return PLAT_BOOT_STATE_APP_VALID;
}

int main(void)
{
    board_init();
    boot_state = select_boot_state();
    tx_kernel_enter();
    return 0;
}

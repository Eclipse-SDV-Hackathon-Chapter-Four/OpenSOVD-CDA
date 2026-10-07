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
 * Minimal newlib system calls. The heap is the fixed region reserved by the
 * linker script; malloc fails instead of growing into the main stack.
 */

#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/lock.h>
#include <sys/stat.h>

#include "tx_api.h"

extern uint8_t __heap_start__[];
extern uint8_t __heap_end__[];

void* _sbrk(ptrdiff_t increment)
{
    static uint8_t* heap_top = __heap_start__;

    TX_INTERRUPT_SAVE_AREA
    TX_DISABLE
    uint8_t* previous = heap_top;
    if (increment > __heap_end__ - heap_top || increment < __heap_start__ - heap_top)
    {
        TX_RESTORE
        errno = ENOMEM;
        return (void*)-1;
    }
    heap_top += increment;
    TX_RESTORE
    return previous;
}

/* newlib locks (malloc is called from several threads by the WICED driver).
 * All newlib locks map onto one recursive ThreadX mutex; the individual lock
 * objects are ignored. */
static TX_MUTEX newlib_mutex;
static int newlib_mutex_ready;

void syscalls_rtos_init(void)
{
    tx_mutex_create(&newlib_mutex, "newlib", TX_INHERIT);
    newlib_mutex_ready = 1;
}

static int locking_needed(void)
{
    /* Before the kernel runs everything is single-threaded. */
    return newlib_mutex_ready && tx_thread_identify() != TX_NULL;
}

void __retarget_lock_acquire_recursive(_LOCK_T lock)
{
    (void)lock;
    if (locking_needed())
    {
        tx_mutex_get(&newlib_mutex, TX_WAIT_FOREVER);
    }
}

void __retarget_lock_release_recursive(_LOCK_T lock)
{
    (void)lock;
    if (locking_needed())
    {
        tx_mutex_put(&newlib_mutex);
    }
}

void __retarget_lock_acquire(_LOCK_T lock)
{
    __retarget_lock_acquire_recursive(lock);
}

void __retarget_lock_release(_LOCK_T lock)
{
    __retarget_lock_release_recursive(lock);
}

int __retarget_lock_try_acquire_recursive(_LOCK_T lock)
{
    (void)lock;
    if (!locking_needed())
    {
        return 1;
    }
    return tx_mutex_get(&newlib_mutex, TX_NO_WAIT) == TX_SUCCESS;
}

int __retarget_lock_try_acquire(_LOCK_T lock)
{
    return __retarget_lock_try_acquire_recursive(lock);
}

/* Dynamic locks (FILE streams) share the global mutex too. */
void __retarget_lock_init(_LOCK_T* lock)
{
    *lock = NULL;
}

void __retarget_lock_init_recursive(_LOCK_T* lock)
{
    *lock = NULL;
}

void __retarget_lock_close(_LOCK_T lock)
{
    (void)lock;
}

void __retarget_lock_close_recursive(_LOCK_T lock)
{
    (void)lock;
}

/* newlib's static lock objects. Defining all symbols of newlib's lock.o keeps
 * that object (with its no-op functions) out of the link. */
struct __lock
{
    char unused;
};
struct __lock __lock___sfp_recursive_mutex;
struct __lock __lock___atexit_recursive_mutex;
struct __lock __lock___at_quick_exit_mutex;
struct __lock __lock___malloc_recursive_mutex;
struct __lock __lock___env_recursive_mutex;
struct __lock __lock___tz_mutex;
struct __lock __lock___dd_hash_mutex;
struct __lock __lock___arc4random_mutex;

int _close(int file)
{
    (void)file;
    return -1;
}

int _fstat(int file, struct stat* st)
{
    (void)file;
    st->st_mode = S_IFCHR;
    return 0;
}

int _isatty(int file)
{
    (void)file;
    return 1;
}

int _lseek(int file, int ptr, int dir)
{
    (void)file;
    (void)ptr;
    (void)dir;
    return 0;
}

int _read(int file, char* ptr, int len)
{
    (void)file;
    (void)ptr;
    (void)len;
    return 0;
}

void _exit(int status)
{
    (void)status;
    for (;;)
    {
    }
}

int _kill(int pid, int sig)
{
    (void)pid;
    (void)sig;
    errno = EINVAL;
    return -1;
}

int _getpid(void)
{
    return 1;
}

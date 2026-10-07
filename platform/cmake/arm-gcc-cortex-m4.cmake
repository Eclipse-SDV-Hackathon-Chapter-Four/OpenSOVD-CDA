# SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Toolchain: Arm GNU Toolchain (arm-none-eabi) for the STM32F412 (Cortex-M4F).
# Uses ./.toolchain (scripts/setup-toolchain.sh) when present, else PATH.

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)

set(THREADX_ARCH "cortex_m4")
set(THREADX_TOOLCHAIN "gnu")

file(GLOB _local_toolchain "${CMAKE_CURRENT_LIST_DIR}/../../.toolchain/arm-gnu-toolchain-*/bin")
if(_local_toolchain)
    list(GET _local_toolchain 0 _bin)
    set(_prefix "${_bin}/arm-none-eabi-")
else()
    set(_prefix "arm-none-eabi-")
endif()

set(CMAKE_C_COMPILER "${_prefix}gcc")
set(CMAKE_ASM_COMPILER "${_prefix}gcc")
set(CMAKE_OBJCOPY "${_prefix}objcopy" CACHE FILEPATH "objcopy")
set(CMAKE_SIZE "${_prefix}size" CACHE FILEPATH "size")

set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(_cpu "-mthumb -mcpu=cortex-m4 -mfloat-abi=hard -mfpu=fpv4-sp-d16")
set(CMAKE_C_FLAGS_INIT "${_cpu} -fdata-sections -ffunction-sections")
set(CMAKE_ASM_FLAGS_INIT "${_cpu} -x assembler-with-cpp")
set(CMAKE_EXE_LINKER_FLAGS_INIT "${_cpu} --specs=nano.specs -Wl,--gc-sections")

set(CMAKE_C_FLAGS_DEBUG "-Og -g3")
set(CMAKE_C_FLAGS_RELEASE "-Os -g")
set(CMAKE_C_FLAGS_MINSIZEREL "-Os -g")

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

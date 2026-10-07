# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# AZ3166 ECU firmware for the MXCHIP AZ3166: C platform (STM32 HAL, ThreadX,
# NetX Duo, WICED Wi-Fi) + Rust ECU (crates/az3166-fw, built with cargo).
#
#   cmake -S platform -B build -G Ninja \
#         -DCMAKE_TOOLCHAIN_FILE=platform/cmake/arm-gcc-cortex-m4.cmake \
#         -DWIFI_SSID=... -DWIFI_PASSWORD=...
#   cmake --build build

cmake_minimum_required(VERSION 3.20)

if(NOT CMAKE_TOOLCHAIN_FILE)
    set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_LIST_DIR}/cmake/arm-gcc-cortex-m4.cmake")
endif()
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE MinSizeRel)
endif()

project(az3166 C ASM)
set(CMAKE_C_STANDARD 11)

set(REPO_DIR "${CMAKE_CURRENT_LIST_DIR}/..")
set(TP "${REPO_DIR}/third_party")
set(GSG "${TP}/getting-started/MXChip/AZ3166")

if(NOT EXISTS "${TP}/threadx/CMakeLists.txt" OR NOT EXISTS "${GSG}")
    message(FATAL_ERROR "Dependencies missing: run scripts/fetch-deps.sh")
endif()

set(WIFI_SSID "" CACHE STRING "Wi-Fi SSID")
set(WIFI_PASSWORD "" CACHE STRING "Wi-Fi WPA2 passphrase (empty = open network)")
set(PRESENCE_SENSOR_HOST "169.254.200.10" CACHE STRING "Presence sensor IPv4 address (HTTP)")
set(PRESENCE_SENSOR_PATH "/binary_sensor/PIR" CACHE STRING "Presence sensor occupancy URL path")

# ---- ThreadX and NetX Duo -----------------------------------------------------
# The configs are the ones the prebuilt WICED library was built with: their
# struct layouts must match.

# The pinned ThreadX/NetX Duo still declare cmake_minimum_required(3.0).
set(CMAKE_POLICY_VERSION_MINIMUM 3.5)

set(TX_USER_FILE "${GSG}/lib/threadx/tx_user.h" CACHE STRING "" FORCE)
set(NX_USER_FILE "${GSG}/lib/netxduo/nx_user.h" CACHE STRING "" FORCE)
set(NXD_ENABLE_FILE_SERVERS OFF CACHE BOOL "" FORCE)
set(NXD_ENABLE_AZURE_IOT OFF CACHE BOOL "" FORCE)

add_subdirectory("${TP}/threadx" threadx EXCLUDE_FROM_ALL)
add_subdirectory("${TP}/netxduo" netxduo EXCLUDE_FROM_ALL)

# Third-party code: not our warnings to fix.
target_compile_options(threadx PRIVATE -w)
target_compile_options(netxduo PRIVATE -w)

# ---- STM32CubeF4 HAL ------------------------------------------------------------

set(HAL_DIR "${GSG}/lib/stm32cubef4/Drivers/STM32F4xx_HAL_Driver")
set(HAL_MODULES
    hal hal_cortex hal_dma hal_dma_ex hal_flash hal_flash_ex hal_gpio hal_i2c hal_i2c_ex
    hal_pwr hal_pwr_ex hal_rcc hal_rcc_ex hal_tim hal_tim_ex hal_uart)
list(TRANSFORM HAL_MODULES PREPEND "${HAL_DIR}/Src/stm32f4xx_" OUTPUT_VARIABLE HAL_SOURCES)
list(TRANSFORM HAL_SOURCES APPEND ".c")

add_library(stm32hal STATIC
    ${HAL_SOURCES}
    "${GSG}/app/startup/system_stm32f4xx.c")
target_compile_definitions(stm32hal PUBLIC STM32F412Rx STM32F4 USE_HAL_DRIVER)
target_include_directories(stm32hal PUBLIC
    "${GSG}/lib/stm32cubef4/config"
    "${HAL_DIR}/Inc"
    "${GSG}/lib/stm32cubef4/Drivers/CMSIS/Device/ST/STM32F4xx/Include"
    "${GSG}/lib/stm32cubef4/Drivers/CMSIS/Include")
target_compile_options(stm32hal PRIVATE -w)

# ---- AZ3166 sensor and OLED drivers -------------------------------------------

set(BSP_DIR "${GSG}/lib/mxchip_bsp")
add_library(mxchip_bsp STATIC
    "${BSP_DIR}/stm_sensor/Src/hts221_reg.c"
    "${BSP_DIR}/stm_sensor/Src/hts221_read_data_polling.c"
    "${BSP_DIR}/stm_sensor/Src/lps22hb_reg.c"
    "${BSP_DIR}/stm_sensor/Src/lps22hb_read_data_polling.c"
    "${BSP_DIR}/stm_sensor/Src/lsm6dsl_reg.c"
    "${BSP_DIR}/stm_sensor/Src/lsm6dsl_read_data_polling.c"
    "${BSP_DIR}/stm_sensor/Src/lis2mdl_reg.c"
    "${BSP_DIR}/stm_sensor/Src/lis2mdl_read_data_polling.c"
    "${BSP_DIR}/ssd1306/ssd1306.c"
    "${BSP_DIR}/ssd1306/ssd1306_fonts.c")
target_include_directories(mxchip_bsp PUBLIC "${BSP_DIR}/stm_sensor/Inc" "${BSP_DIR}/ssd1306")
target_compile_definitions(mxchip_bsp PUBLIC SSD1306_INCLUDE_FONT_7x10)
target_link_libraries(mxchip_bsp PUBLIC stm32hal)
target_compile_options(mxchip_bsp PRIVATE -w)

# ---- WICED Wi-Fi (prebuilt, Cypress license) --------------------------------------

add_library(wiced_sdk STATIC IMPORTED)
set_target_properties(wiced_sdk PROPERTIES
    IMPORTED_LOCATION "${GSG}/lib/wiced_sdk/lib/libwiced_sdk_bin.a"
    INTERFACE_INCLUDE_DIRECTORIES "${GSG}/lib/wiced_sdk/inc")

# ---- Rust firmware -----------------------------------------------------------------

set(RUST_TARGET thumbv7em-none-eabihf)
set(RUST_LIB "${REPO_DIR}/target/${RUST_TARGET}/release/libaz3166_fw.a")
file(GLOB_RECURSE RUST_SOURCES CONFIGURE_DEPENDS
    "${REPO_DIR}/crates/*.rs" "${REPO_DIR}/crates/*/Cargo.toml")
add_custom_command(
    OUTPUT "${RUST_LIB}"
    COMMAND cargo build --release --target ${RUST_TARGET} -p az3166-fw
    WORKING_DIRECTORY "${REPO_DIR}"
    DEPENDS ${RUST_SOURCES} "${REPO_DIR}/Cargo.toml"
    COMMENT "Building Rust firmware (cargo)"
    USES_TERMINAL)
add_custom_target(rust_fw DEPENDS "${RUST_LIB}")
add_library(az3166_fw STATIC IMPORTED)
set_target_properties(az3166_fw PROPERTIES IMPORTED_LOCATION "${RUST_LIB}")
add_dependencies(az3166_fw rust_fw)

# ---- Firmware images -----------------------------------------------------------------
#
#   az3166-boot.elf    bootloader at 0x08000000 (Boot variant + slot selection)
#   az3166-app-a.elf   app linked for slot A (0x08060000)
#   az3166-app-b.elf   app linked for slot B (0x080A0000)
#
#   az3166-factory.bin   1 MiB: bootloader + app in slot A + erased rest
#                        (first flash over USB; resets the update state)
#   update/az3166-app-<version>.bin
#                        app update package for the CDA (both slots)

set(APP_VERSION "0.2.0" CACHE STRING "App firmware version (display, DID F195, package)")
set(BOOT_VERSION "0.2.0" CACHE STRING "Bootloader version")
option(APP_TEST_HANG "Test only: app hangs at startup (exercises the rollback)" OFF)

add_library(platform_common OBJECT
    src/board.c
    src/boot_state.c
    src/msp.c
    src/net.c
    src/syscalls.c
    src/update.c
    src/watchdog.c
    "${GSG}/app/startup/startup_stm32f412rx.s"
    "${GSG}/app/startup/tx_initialize_low_level.S")
target_include_directories(platform_common PUBLIC src)
target_link_libraries(platform_common PUBLIC netxduo threadx wiced_sdk mxchip_bsp stm32hal)
target_compile_options(platform_common PRIVATE $<$<COMPILE_LANGUAGE:C>:-Wall -Wextra -Werror>)
target_compile_definitions(platform_common PRIVATE
    "PRESENCE_SENSOR_HOST=\"${PRESENCE_SENSOR_HOST}\"" "PRESENCE_SENSOR_PATH=\"${PRESENCE_SENSOR_PATH}\"")

set(LINKER_DIR "${CMAKE_CURRENT_LIST_DIR}/linker")

function(add_firmware_image name linker_script)
    add_executable(${name} ${ARGN} $<TARGET_OBJECTS:platform_common>)
    set_target_properties(${name} PROPERTIES SUFFIX ".elf")
    target_include_directories(${name} PRIVATE src)
    target_compile_options(${name} PRIVATE $<$<COMPILE_LANGUAGE:C>:-Wall -Wextra -Werror>)
    target_compile_definitions(${name} PRIVATE
        "WIFI_SSID=\"${WIFI_SSID}\"" "WIFI_PASSWORD=\"${WIFI_PASSWORD}\"")
    # The Rust library calls into the platform and vice versa: group them.
    target_link_libraries(${name} PRIVATE
        -Wl,--start-group
        az3166_fw netxduo threadx wiced_sdk mxchip_bsp stm32hal
        -Wl,--end-group
        m)
    target_link_options(${name} PRIVATE
        "-L${LINKER_DIR}"
        "-T${LINKER_DIR}/${linker_script}"
        "-Wl,-Map=${CMAKE_BINARY_DIR}/${name}.map"
        "-Wl,--print-memory-usage")
    file(GLOB _ld "${LINKER_DIR}/*.ld")
    set_property(TARGET ${name} APPEND PROPERTY LINK_DEPENDS ${_ld})
    add_custom_command(TARGET ${name} POST_BUILD
        COMMAND ${CMAKE_OBJCOPY} -O binary ${name}.elf ${name}.bin
        COMMAND ${CMAKE_SIZE} ${name}.elf
        WORKING_DIRECTORY "${CMAKE_BINARY_DIR}"
        VERBATIM)
endfunction()

add_firmware_image(az3166-boot boot.ld src/main.c src/bootloader.c src/audio_stub.c)
target_compile_definitions(az3166-boot PRIVATE FW_IMAGE_BOOT "FW_VERSION=\"${BOOT_VERSION}\"")

foreach(slot a b)
    add_firmware_image(az3166-app-${slot} app_${slot}.ld src/main.c src/audio.c)
    target_compile_definitions(az3166-app-${slot} PRIVATE FW_IMAGE_APP "FW_VERSION=\"${APP_VERSION}\"")
    if(APP_TEST_HANG)
        target_compile_definitions(az3166-app-${slot} PRIVATE APP_TEST_HANG)
    endif()
endforeach()

set(PACKAGE "${CMAKE_BINARY_DIR}/update/az3166-app-${APP_VERSION}.bin")
add_custom_command(
    OUTPUT "${CMAKE_BINARY_DIR}/az3166-factory.bin" "${PACKAGE}"
    COMMAND python3 -I "${REPO_DIR}/scripts/make-images.py"
        --version "${APP_VERSION}"
        --boot az3166-boot.bin --app-a az3166-app-a.bin --app-b az3166-app-b.bin
        --factory az3166-factory.bin --package "${PACKAGE}"
    DEPENDS az3166-boot az3166-app-a az3166-app-b "${REPO_DIR}/scripts/make-images.py"
    WORKING_DIRECTORY "${CMAKE_BINARY_DIR}"
    COMMENT "Packaging factory image and app update"
    VERBATIM)
add_custom_target(images ALL DEPENDS "${CMAKE_BINARY_DIR}/az3166-factory.bin" "${PACKAGE}")

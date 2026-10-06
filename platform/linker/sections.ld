/*
 * SPDX-License-Identifier: MIT
 *
 * Output sections shared by the bootloader and app images of the MXCHIP
 * AZ3166 (STM32F412RG). INCLUDEd inside SECTIONS by boot.ld / app_*.ld,
 * which define the FLASH and RAM regions.
 * Derived from MXChip_AZ3166.ld of the Azure RTOS getting-started guides,
 * Copyright (c) Microsoft Corporation.
 *
 * Changes: full 256 KiB SRAM, explicit heap and main-stack regions, no CCM
 * RAM (F412 has none).
 */

  .isr_vector :
  {
    . = ALIGN(4);
    KEEP(*(.isr_vector))
    . = ALIGN(4);
  } >FLASH

  /* Image header at a fixed offset after the vector table (image_info.h). */
  .image_info :
  {
    . = ALIGN(0x200);
    KEEP(*(.image_info))
    . = ALIGN(4);
  } >FLASH

  .text :
  {
    . = ALIGN(4);
    *(.text)
    *(.text*)
    *(.glue_7)
    *(.glue_7t)
    *(.eh_frame)
    KEEP (*(.init))
    KEEP (*(.fini))
    . = ALIGN(4);
    _etext = .;
  } >FLASH

  .rodata :
  {
    . = ALIGN(4);
    *(.rodata)
    *(.rodata*)
    . = ALIGN(4);
  } >FLASH

  .ARM.extab : { *(.ARM.extab* .gnu.linkonce.armextab.*) } >FLASH
  .ARM :
  {
    __exidx_start = .;
    *(.ARM.exidx*)
    __exidx_end = .;
  } >FLASH

  .preinit_array :
  {
    PROVIDE_HIDDEN (__preinit_array_start = .);
    KEEP (*(.preinit_array*))
    PROVIDE_HIDDEN (__preinit_array_end = .);
  } >FLASH
  .init_array :
  {
    PROVIDE_HIDDEN (__init_array_start = .);
    KEEP (*(SORT(.init_array.*)))
    KEEP (*(.init_array*))
    PROVIDE_HIDDEN (__init_array_end = .);
  } >FLASH
  .fini_array :
  {
    PROVIDE_HIDDEN (__fini_array_start = .);
    KEEP (*(SORT(.fini_array.*)))
    KEEP (*(.fini_array*))
    PROVIDE_HIDDEN (__fini_array_end = .);
  } >FLASH

  _sidata = LOADADDR(.data);
  .data :
  {
    . = ALIGN(4);
    _sdata = .;
    *(.data)
    *(.data*)
    . = ALIGN(4);
    _edata = .;
  } >RAM AT> FLASH

  . = ALIGN(4);
  .bss (NOLOAD) :
  {
    _sbss = .;
    __bss_start__ = _sbss;
    *(.bss)
    *(.bss*)
    *(COMMON)
    . = ALIGN(4);
    _ebss = .;
    __bss_end__ = _ebss;
  } >RAM

  .heap (NOLOAD) :
  {
    . = ALIGN(8);
    PROVIDE ( end = . );
    PROVIDE ( _end = . );
    __heap_start__ = .;
    . = . + _Heap_Size;
    . = ALIGN(8);
    __heap_end__ = .;
  } >RAM

  /* Main stack (MSP), right above the heap. The link fails if .bss, heap
     and main stack do not fit. RAM above it is left to ThreadX. */
  .stack (NOLOAD) :
  {
    . = ALIGN(8);
    . = . + _Stack_Size;
    . = ALIGN(8);
    _estack = .;
    __RAM_segment_used_end__ = .;   /* ThreadX first unused memory */
  } >RAM

  /DISCARD/ :
  {
    libc.a ( * )
    libm.a ( * )
    libgcc.a ( * )
  }

  .ARM.attributes 0 : { *(.ARM.attributes) }

/* SPDX-License-Identifier: MIT
 * Linker script for Crispy Bootloader C++ firmware - RAM execution
 *
 * All code and data in one contiguous 192KB region starting at 0x20000000.
 * The bootloader copies 192KB from flash to this address before jumping.
 * Vector table MUST be at offset 0 for bootloader validation.
 */

MEMORY
{
    /* Single RAM region for code + data (192KB max) */
    RAM(rwx) : ORIGIN = 0x20000000, LENGTH = 192k
    /* Scratch areas for stacks */
    SCRATCH_X(rwx) : ORIGIN = 0x20040000, LENGTH = 4k
    SCRATCH_Y(rwx) : ORIGIN = 0x20041000, LENGTH = 4k
}

ENTRY(_entry_point)

SECTIONS
{
    /* CRITICAL: Vector table MUST be first at offset 0 for Crispy bootloader */
    .vectors : {
        __logical_binary_start = .;
        __flash_binary_start = .;
        KEEP (*(.vectors))
    } > RAM

    .reset : {
        __reset_start = .;
        KEEP (*(.reset))
        __reset_end = .;
    } > RAM

    .binary_info_header : ALIGN(4) {
        KEEP (*(.binary_info_header))
        __binary_info_header_end = .;
    } > RAM

    .text : {
        *(.time_critical*)
        *(.text*)
        . = ALIGN(4);
        *(.init)
        *(.fini)
        *crtbegin.o(.ctors)
        *crtbegin?.o(.ctors)
        *(EXCLUDE_FILE(*crtend?.o *crtend.o) .ctors)
        *(SORT(.ctors.*))
        *(.ctors)
        *crtbegin.o(.dtors)
        *crtbegin?.o(.dtors)
        *(EXCLUDE_FILE(*crtend?.o *crtend.o) .dtors)
        *(SORT(.dtors.*))
        *(.dtors)
        *(.eh_frame*)
    } > RAM

    .rodata : {
        . = ALIGN(4);
        *(.rodata*)
        . = ALIGN(4);
        *(SORT_BY_ALIGNMENT(SORT_BY_NAME(.flashdata*)))
        . = ALIGN(4);
    } > RAM

    .ARM.extab : { *(.ARM.extab* .gnu.linkonce.armextab.*) } > RAM

    __exidx_start = .;
    .ARM.exidx : { *(.ARM.exidx* .gnu.linkonce.armexidx.*) } > RAM
    __exidx_end = .;

    . = ALIGN(4);
    __binary_info_start = .;
    .binary_info : {
        KEEP(*(.binary_info.keep.*))
        *(.binary_info.*)
    } > RAM
    __binary_info_end = .;

    __etext = .;

    .data : {
        __data_start__ = .;
        *(vtable)
        *(.data*)
        . = ALIGN(4);
        *(.after_data.*)
        PROVIDE_HIDDEN (__mutex_array_start = .);
        KEEP(*(SORT(.mutex_array.*)))
        KEEP(*(.mutex_array))
        PROVIDE_HIDDEN (__mutex_array_end = .);
        PROVIDE_HIDDEN (__preinit_array_start = .);
        KEEP(*(SORT(.preinit_array.*)))
        KEEP(*(.preinit_array))
        PROVIDE_HIDDEN (__preinit_array_end = .);
        PROVIDE_HIDDEN (__init_array_start = .);
        KEEP(*(SORT(.init_array.*)))
        KEEP(*(.init_array))
        PROVIDE_HIDDEN (__init_array_end = .);
        PROVIDE_HIDDEN (__fini_array_start = .);
        *(SORT(.fini_array.*))
        *(.fini_array)
        PROVIDE_HIDDEN (__fini_array_end = .);
        *(.jcr)
        . = ALIGN(4);
        __data_end__ = .;
    } > RAM

    .uninitialized_data (NOLOAD): { . = ALIGN(4); *(.uninitialized_data*) } > RAM

    .bss (NOLOAD) : {
        . = ALIGN(4);
        __bss_start__ = .;
        *(SORT_BY_ALIGNMENT(SORT_BY_NAME(.bss*)))
        *(COMMON)
        . = ALIGN(4);
        __bss_end__ = .;
    } > RAM

    .heap (NOLOAD): {
        __end__ = .;
        end = __end__;
        KEEP(*(.heap*))
    } > RAM

    __HeapLimit = ORIGIN(RAM) + LENGTH(RAM);

    /* Stacks in scratch memory */
    .stack1_dummy (NOLOAD): { *(.stack1*) } > SCRATCH_X
    .stack_dummy (NOLOAD): { KEEP(*(.stack*)) } > SCRATCH_Y

    __StackLimit = ORIGIN(SCRATCH_Y);
    __StackOneTop = ORIGIN(SCRATCH_X) + LENGTH(SCRATCH_X);
    __StackTop = ORIGIN(SCRATCH_Y) + LENGTH(SCRATCH_Y);
    __StackOneBottom = __StackOneTop - SIZEOF(.stack1_dummy);
    __StackBottom = __StackTop - SIZEOF(.stack_dummy);
    PROVIDE(__stack = __StackTop);

    PROVIDE (__heap_start = __end__);
    PROVIDE (__heap_end = __HeapLimit);
    PROVIDE (__flash_binary_end = __binary_info_end);

    .scratch_x : { __scratch_x_start__ = .; *(.scratch_x.*) . = ALIGN(4); __scratch_x_end__ = .; } > SCRATCH_X
    __scratch_x_source__ = LOADADDR(.scratch_x);

    .scratch_y : { __scratch_y_start__ = .; *(.scratch_y.*) . = ALIGN(4); __scratch_y_end__ = .; } > SCRATCH_Y
    __scratch_y_source__ = LOADADDR(.scratch_y);

    ASSERT(__HeapLimit >= __bss_end__, "RAM region overflowed")
}

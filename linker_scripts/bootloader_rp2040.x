/*
* SPDX-License-Identifier: MIT OR Apache-2.0
* Bootloader linker script for RP2040
*
* RAM layout (256KB):
*   0x20000000 - 0x20030000: Firmware code (192KB, copied by bootloader)
*   0x20030000 - 0x2003C000: Firmware data/BSS/stack (48KB)
*   0x2003C000 - 0x20040000: Bootloader data/BSS/stack (16KB)
*/

/* =========================== MEMORY LAYOUT CONFIG =========================== */
/* Modify these values to change memory allocation (must be 4KB sector-aligned) */

__flash_base       = 0x10000000;
__boot2_size       = 0x100;      /* 256B - fixed by RP2040 */
__bootloader_size  = 0x10000;    /* 64KB - adjust as needed */
__fw_bank_size     = 0xC0000;    /* 768KB per firmware bank */
__boot_data_size   = 0x1000;     /* 4KB for boot metadata */
__fw_copy_size     = 0x30000;    /* 192KB copied to RAM */

/* Bootloader RAM (top of SRAM) */
__bootloader_ram   = 0x2003C000;
__bootloader_ram_size = 16K;

/* Firmware RAM base (copied from flash) */
__fw_ram_base      = 0x20000000;

/* Valid RAM range for firmware validation (includes SCRATCH areas for stack) */
__fw_ram_start     = 0x20000000;
__fw_ram_end       = 0x20042000;

/* ============================================================================ */

/* Calculated addresses (do not modify) */
__fw_a_entry       = __flash_base + __bootloader_size;
__fw_b_entry       = __fw_a_entry + __fw_bank_size;
__boot_data_addr   = __fw_b_entry + __fw_bank_size;

MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = __boot2_size
    FLASH : ORIGIN = 0x10000000 + __boot2_size, LENGTH = __bootloader_size - __boot2_size
    RAM   : ORIGIN = __bootloader_ram, LENGTH = __bootloader_ram_size
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;

SECTIONS {
    /* ### Boot ROM info */
    .boot_info : ALIGN(4)
    {
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.boot_info) + SIZEOF(.boot_info);

SECTIONS {
    /* ### Picotool 'Binary Info' Entries */
    .bi_entries : ALIGN(4)
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries));
        . = ALIGN(4);
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

/* Export symbols for bootloader code */
PROVIDE(__fw_a_entry = __fw_a_entry);
PROVIDE(__fw_b_entry = __fw_b_entry);
PROVIDE(__boot_data_addr = __boot_data_addr);
PROVIDE(__fw_ram_base = __fw_ram_base);
PROVIDE(__fw_copy_size = __fw_copy_size);
PROVIDE(__fw_ram_start = __fw_ram_start);
PROVIDE(__fw_ram_end = __fw_ram_end);

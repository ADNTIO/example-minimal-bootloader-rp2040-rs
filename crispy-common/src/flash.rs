// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Flash operations for firmware - read/write BootData, program firmware banks.
//!
//! This module provides flash operations that can be used by firmware to:
//! - Confirm boot (write confirmed=1 to BootData)
//! - Write firmware to banks (self-update capability)
//! - Manage boot configuration

use crate::protocol::{
    BootData, BOOT_DATA_ADDR, FLASH_BASE, FLASH_PAGE_SIZE, FLASH_SECTOR_SIZE,
    FW_A_ADDR, FW_B_ADDR, FW_BANK_SIZE, RAM_UPDATE_FLAG_ADDR, RAM_UPDATE_MAGIC,
};

/// Read BootData from flash.
pub fn read_boot_data() -> BootData {
    unsafe { BootData::read_from(BOOT_DATA_ADDR) }
}

/// Write BootData to flash.
///
/// # Safety
/// Caller must ensure no code is executing from flash during this operation.
pub unsafe fn write_boot_data(bd: &BootData) {
    let offset = BOOT_DATA_ADDR - FLASH_BASE;

    // Pad to page size
    let mut page = [0xFFu8; FLASH_PAGE_SIZE as usize];
    let src = bd.as_bytes();
    page[..src.len()].copy_from_slice(src);

    flash_erase_and_program(offset, &page);
}

/// Confirm the current boot to the bootloader.
/// Sets confirmed=1 and boot_attempts=0 in BootData.
///
/// Returns true if confirmation was successful, false if BootData is invalid.
pub fn confirm_boot() -> bool {
    let mut bd = read_boot_data();

    if !bd.is_valid() {
        return false;
    }

    if bd.confirmed == 1 {
        return true; // Already confirmed
    }

    bd.confirmed = 1;
    bd.boot_attempts = 0;

    unsafe {
        write_boot_data(&bd);
    }

    true
}

/// Set the active bank for next boot.
///
/// # Arguments
/// * `bank` - 0 for bank A, 1 for bank B
///
/// Returns false if bank is invalid or BootData is invalid.
pub fn set_active_bank(bank: u8) -> bool {
    if bank > 1 {
        return false;
    }

    let mut bd = read_boot_data();
    if !bd.is_valid() {
        bd = BootData::default_new();
    }

    bd.active_bank = bank;
    bd.confirmed = 0;
    bd.boot_attempts = 0;

    unsafe {
        write_boot_data(&bd);
    }

    true
}

/// Get the flash address for a bank.
pub fn bank_address(bank: u8) -> u32 {
    if bank == 0 {
        FW_A_ADDR
    } else {
        FW_B_ADDR
    }
}

/// Get the inactive bank (opposite of current active bank).
pub fn inactive_bank() -> u8 {
    let bd = read_boot_data();
    if bd.is_valid() && bd.active_bank == 0 {
        1
    } else {
        0
    }
}

/// Erase a firmware bank.
///
/// # Arguments
/// * `bank` - 0 for bank A, 1 for bank B
///
/// # Safety
/// Caller must ensure no code is executing from the target bank.
pub unsafe fn erase_bank(bank: u8) {
    let addr = bank_address(bank);
    let offset = addr - FLASH_BASE;

    // Erase entire bank (768KB = 192 sectors of 4KB)
    let num_sectors = FW_BANK_SIZE / FLASH_SECTOR_SIZE;

    cortex_m::interrupt::disable();
    rp2040_hal::rom_data::connect_internal_flash();
    rp2040_hal::rom_data::flash_exit_xip();

    for i in 0..num_sectors {
        let sector_offset = offset + i * FLASH_SECTOR_SIZE;
        rp2040_hal::rom_data::flash_range_erase(
            sector_offset,
            FLASH_SECTOR_SIZE as usize,
            FLASH_SECTOR_SIZE,
            0x20, // SECTOR_ERASE command
        );
    }

    rp2040_hal::rom_data::flash_flush_cache();
    rp2040_hal::rom_data::flash_enter_cmd_xip();
    cortex_m::interrupt::enable();
}

/// Write data to a firmware bank at the specified offset.
///
/// # Arguments
/// * `bank` - 0 for bank A, 1 for bank B
/// * `offset` - Offset within the bank (must be page-aligned, 256 bytes)
/// * `data` - Data to write (must be page-aligned length)
///
/// # Safety
/// Caller must ensure:
/// - No code is executing from the target bank
/// - The bank has been erased before writing
/// - Offset + data.len() <= FW_BANK_SIZE
pub unsafe fn write_to_bank(bank: u8, offset: u32, data: &[u8]) {
    let bank_addr = bank_address(bank);
    let flash_offset = (bank_addr - FLASH_BASE) + offset;

    cortex_m::interrupt::disable();
    rp2040_hal::rom_data::connect_internal_flash();
    rp2040_hal::rom_data::flash_exit_xip();
    rp2040_hal::rom_data::flash_range_program(flash_offset, data.as_ptr(), data.len());
    rp2040_hal::rom_data::flash_flush_cache();
    rp2040_hal::rom_data::flash_enter_cmd_xip();
    cortex_m::interrupt::enable();
}

/// Update firmware metadata in BootData after writing firmware to a bank.
///
/// # Arguments
/// * `bank` - 0 for bank A, 1 for bank B
/// * `size` - Firmware size in bytes
/// * `crc` - CRC32 of the firmware
/// * `version` - Firmware version number
pub fn update_bank_metadata(bank: u8, size: u32, crc: u32, version: u32) {
    let mut bd = read_boot_data();
    if !bd.is_valid() {
        bd = BootData::default_new();
    }

    if bank == 0 {
        bd.size_a = size;
        bd.crc_a = crc;
        bd.version_a = version;
    } else {
        bd.size_b = size;
        bd.crc_b = crc;
        bd.version_b = version;
    }

    unsafe {
        write_boot_data(&bd);
    }
}

/// Compute CRC32 of data in flash.
pub fn compute_crc32(addr: u32, size: u32) -> u32 {
    let data = unsafe { core::slice::from_raw_parts(addr as *const u8, size as usize) };

    // CRC32 (same polynomial as used by bootloader)
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Reboot to bootloader update mode.
///
/// This writes the magic flag to RAM and triggers a system reset.
/// The bootloader will detect the flag and enter update mode.
pub fn reboot_to_bootloader() -> ! {
    unsafe {
        (RAM_UPDATE_FLAG_ADDR as *mut u32).write_volatile(RAM_UPDATE_MAGIC);
    }

    // Small delay to ensure write completes
    cortex_m::asm::delay(100_000);

    cortex_m::peripheral::SCB::sys_reset();
}

/// Reboot normally.
pub fn reboot() -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

// --- Internal helpers ---

unsafe fn flash_erase_and_program(offset: u32, data: &[u8]) {
    cortex_m::interrupt::disable();

    rp2040_hal::rom_data::connect_internal_flash();
    rp2040_hal::rom_data::flash_exit_xip();
    rp2040_hal::rom_data::flash_range_erase(
        offset,
        FLASH_SECTOR_SIZE as usize,
        FLASH_SECTOR_SIZE,
        0x20,
    );
    rp2040_hal::rom_data::flash_flush_cache();
    rp2040_hal::rom_data::flash_enter_cmd_xip();

    rp2040_hal::rom_data::connect_internal_flash();
    rp2040_hal::rom_data::flash_exit_xip();
    rp2040_hal::rom_data::flash_range_program(offset, data.as_ptr(), data.len());
    rp2040_hal::rom_data::flash_flush_cache();
    rp2040_hal::rom_data::flash_enter_cmd_xip();

    cortex_m::interrupt::enable();
}

// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Flash read/write/erase wrappers using RP2040 ROM routines.
//!
//! On RP2040, flash operations (erase/program) require disabling XIP first.
//! The full sequence is:
//!   1. connect_internal_flash()
//!   2. flash_exit_xip()
//!   3. flash_range_erase() or flash_range_program()
//!   4. flash_flush_cache()
//!   5. flash_enter_cmd_xip()
//!
//! All code executing during steps 1-5 must run from RAM, not flash.
//! We use `#[link_section = ".data"]` to place critical functions in RAM,
//! and pre-resolve all ROM function pointers at init time.

use crc::{Crc, CRC_32_ISO_HDLC};
use crispy_common::protocol::{
    BootData, BOOT_DATA_ADDR, FLASH_BASE, FLASH_PAGE_SIZE, FLASH_SECTOR_SIZE,
};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

// ROM function pointer types
type RomFnVoid = unsafe extern "C" fn();
type RomFnErase = unsafe extern "C" fn(u32, usize, u32, u8);
type RomFnProgram = unsafe extern "C" fn(u32, *const u8, usize);

/// ROM function pointers, resolved once at init from the ROM table.
/// Stored in static RAM so RAM-resident functions can call them without
/// accessing flash-based code.
static mut ROM_CONNECT_INTERNAL_FLASH: RomFnVoid = dummy_void;
static mut ROM_FLASH_EXIT_XIP: RomFnVoid = dummy_void;
static mut ROM_FLASH_RANGE_ERASE: RomFnErase = dummy_erase;
static mut ROM_FLASH_RANGE_PROGRAM: RomFnProgram = dummy_program;
static mut ROM_FLASH_FLUSH_CACHE: RomFnVoid = dummy_void;
static mut ROM_FLASH_ENTER_CMD_XIP: RomFnVoid = dummy_void;

unsafe extern "C" fn dummy_void() {}
unsafe extern "C" fn dummy_erase(_: u32, _: usize, _: u32, _: u8) {}
unsafe extern "C" fn dummy_program(_: u32, _: *const u8, _: usize) {}

/// Look up a ROM function by its two-character tag.
/// ROM table pointer at 0x14 and lookup function at 0x18 are 16-bit halfword pointers.
unsafe fn rom_func_lookup(tag: &[u8; 2]) -> usize {
    let fn_table = *(0x14 as *const u16) as *const u16;
    let lookup: unsafe extern "C" fn(*const u16, u32) -> usize =
        core::mem::transmute::<usize, unsafe extern "C" fn(*const u16, u32) -> usize>(
            *(0x18 as *const u16) as usize,
        );
    let code = u16::from_le_bytes(*tag) as u32;
    lookup(fn_table, code)
}

/// Initialize ROM flash function pointers. Must be called once before any flash operations.
/// This performs ROM table lookups which require XIP to be active.
pub fn init() {
    unsafe {
        ROM_CONNECT_INTERNAL_FLASH =
            core::mem::transmute::<usize, RomFnVoid>(rom_func_lookup(b"IF"));
        ROM_FLASH_EXIT_XIP =
            core::mem::transmute::<usize, RomFnVoid>(rom_func_lookup(b"EX"));
        ROM_FLASH_RANGE_ERASE =
            core::mem::transmute::<usize, RomFnErase>(rom_func_lookup(b"RE"));
        ROM_FLASH_RANGE_PROGRAM =
            core::mem::transmute::<usize, RomFnProgram>(rom_func_lookup(b"RP"));
        ROM_FLASH_FLUSH_CACHE =
            core::mem::transmute::<usize, RomFnVoid>(rom_func_lookup(b"FC"));
        ROM_FLASH_ENTER_CMD_XIP =
            core::mem::transmute::<usize, RomFnVoid>(rom_func_lookup(b"CX"));
    }
}

/// Convert an absolute XIP flash address to a flash-relative offset.
pub fn addr_to_offset(abs_addr: u32) -> u32 {
    abs_addr - FLASH_BASE
}

/// Erase flash at the given flash-relative offset.
/// Runs entirely from RAM with proper XIP teardown/setup.
///
/// # Safety
/// The `init()` function must have been called first.
#[link_section = ".data"]
#[inline(never)]
pub unsafe fn flash_erase(offset: u32, size: u32) {
    cortex_m::interrupt::disable();
    ROM_CONNECT_INTERNAL_FLASH();
    ROM_FLASH_EXIT_XIP();
    ROM_FLASH_RANGE_ERASE(offset, size as usize, FLASH_SECTOR_SIZE, 0x20);
    ROM_FLASH_FLUSH_CACHE();
    ROM_FLASH_ENTER_CMD_XIP();
    cortex_m::interrupt::enable();
}

/// Program flash at the given flash-relative offset.
/// Runs entirely from RAM with proper XIP teardown/setup.
///
/// # Safety
/// The `init()` function must have been called first.
#[link_section = ".data"]
#[inline(never)]
pub unsafe fn flash_program(offset: u32, data: *const u8, len: usize) {
    cortex_m::interrupt::disable();
    ROM_CONNECT_INTERNAL_FLASH();
    ROM_FLASH_EXIT_XIP();
    ROM_FLASH_RANGE_PROGRAM(offset, data, len);
    ROM_FLASH_FLUSH_CACHE();
    ROM_FLASH_ENTER_CMD_XIP();
    cortex_m::interrupt::enable();
}

/// Read bytes from an absolute XIP flash address via volatile reads.
pub fn flash_read(abs_addr: u32, buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = unsafe { ((abs_addr + i as u32) as *const u8).read_volatile() };
    }
}

/// Compute CRC-32 (ISO HDLC) over flash data at the given absolute address.
pub fn compute_crc32(abs_addr: u32, size: u32) -> u32 {
    let mut digest = CRC32.digest();
    let mut remaining = size as usize;
    let mut addr = abs_addr;
    let mut chunk = [0u8; 256];

    while remaining > 0 {
        let n = remaining.min(chunk.len());
        flash_read(addr, &mut chunk[..n]);
        digest.update(&chunk[..n]);
        addr += n as u32;
        remaining -= n;
    }

    digest.finalize()
}

/// Read BootData from flash. Returns default if magic is invalid.
pub fn read_boot_data() -> BootData {
    let bd = unsafe { BootData::read_from(BOOT_DATA_ADDR) };
    if bd.is_valid() {
        bd
    } else {
        BootData::default_new()
    }
}

/// Write BootData to flash (erase sector, then program padded to 256B page).
///
/// # Safety
/// The `init()` function must have been called first.
pub unsafe fn write_boot_data(bd: &BootData) {
    let offset = addr_to_offset(BOOT_DATA_ADDR);

    // Erase the 4KB sector containing boot data
    flash_erase(offset, FLASH_SECTOR_SIZE);

    // Pad to a full 256-byte page
    let mut page = [0xFFu8; FLASH_PAGE_SIZE as usize];
    let src = bd.as_bytes();
    page[..src.len()].copy_from_slice(src);

    flash_program(offset, page.as_ptr(), page.len());
}

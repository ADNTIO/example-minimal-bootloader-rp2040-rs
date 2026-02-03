// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Shared protocol types for bootloader <-> host communication.
//!
//! This module provides types that work in both `no_std` (embedded) and `std` (host) environments.
//! Use the `std` feature for host tools.

#[cfg(feature = "std")]
extern crate alloc;

use serde::{Deserialize, Serialize};

// --- Flash layout constants ---

pub const FLASH_BASE: u32 = 0x1000_0000;
pub const FW_A_ADDR: u32 = 0x1001_0000;
pub const FW_B_ADDR: u32 = 0x100D_0000;
pub const BOOT_DATA_ADDR: u32 = 0x1019_0000;

pub const FW_BANK_SIZE: u32 = 768 * 1024; // 768KB per bank

pub const RAM_UPDATE_FLAG_ADDR: u32 = 0x2003_BFF0;
pub const RAM_UPDATE_MAGIC: u32 = 0x0FDA_7E00;

pub const FLASH_SECTOR_SIZE: u32 = 4096;
pub const FLASH_PAGE_SIZE: u32 = 256;

pub const BOOT_DATA_MAGIC: u32 = 0xB007_DA7A;

// --- BootData (repr(C), 32 bytes) ---

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootData {
    pub magic: u32,        // 0xB007DA7A
    pub active_bank: u8,   // 0 = A, 1 = B
    pub confirmed: u8,     // 1 = confirmed good
    pub boot_attempts: u8, // rollback after 3
    pub _reserved0: u8,
    pub version_a: u32, // firmware version in bank A
    pub version_b: u32, // firmware version in bank B
    pub crc_a: u32,     // CRC32 of bank A firmware
    pub crc_b: u32,     // CRC32 of bank B firmware
    pub size_a: u32,    // size of firmware in bank A
    pub size_b: u32,    // size of firmware in bank B
}

// Compile-time size check
const _: () = assert!(core::mem::size_of::<BootData>() == 32);

impl BootData {
    pub fn default_new() -> Self {
        Self {
            magic: BOOT_DATA_MAGIC,
            active_bank: 0,
            confirmed: 0,
            boot_attempts: 0,
            _reserved0: 0,
            version_a: 0,
            version_b: 0,
            crc_a: 0,
            crc_b: 0,
            size_a: 0,
            size_b: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == BOOT_DATA_MAGIC
    }

    pub fn bank_addr(&self) -> u32 {
        if self.active_bank == 0 {
            FW_A_ADDR
        } else {
            FW_B_ADDR
        }
    }

    /// Read BootData from a raw address via volatile reads.
    ///
    /// # Safety
    /// `addr` must point to a readable, properly aligned memory region of at least 32 bytes.
    pub unsafe fn read_from(addr: u32) -> Self {
        let ptr = addr as *const Self;
        core::ptr::read_volatile(ptr)
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

// --- Command / Response protocol ---

/// Maximum data block size for firmware uploads.
pub const MAX_DATA_BLOCK_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::large_enum_variant)] // no_std, no allocator for Box
pub enum Command {
    GetStatus,
    StartUpdate {
        bank: u8,
        size: u32,
        crc32: u32,
        version: u32,
    },
    #[cfg(not(feature = "std"))]
    DataBlock {
        offset: u32,
        data: heapless::Vec<u8, MAX_DATA_BLOCK_SIZE>,
    },
    #[cfg(feature = "std")]
    DataBlock {
        offset: u32,
        data: alloc::vec::Vec<u8>,
    },
    FinishUpdate,
    Reboot,
    /// Set the active bank for the next boot (without uploading firmware).
    SetActiveBank { bank: u8 },
    /// Wipe all firmware banks and reset boot data.
    WipeAll,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ack(AckStatus),
    Status {
        active_bank: u8,
        version_a: u32,
        version_b: u32,
        state: BootState,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    Ok,
    CrcError,
    FlashError,
    BadCommand,
    BadState,
    BankInvalid,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootState {
    Idle,
    UpdateMode,
    Receiving,
}

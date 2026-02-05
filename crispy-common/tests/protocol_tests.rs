// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Unit tests for protocol types and constants.

use crispy_common::protocol::{
    AckStatus, BootState, Command, Response, BOOT_DATA_ADDR, FLASH_BASE, FLASH_PAGE_SIZE,
    FLASH_SECTOR_SIZE, FW_A_ADDR, FW_BANK_SIZE, FW_B_ADDR, MAX_DATA_BLOCK_SIZE,
    RAM_UPDATE_FLAG_ADDR, RAM_UPDATE_MAGIC,
};

// --- Flash layout constants tests ---

#[test]
fn test_flash_base_address() {
    assert_eq!(FLASH_BASE, 0x1000_0000);
}

#[test]
fn test_firmware_bank_addresses() {
    assert_eq!(FW_A_ADDR, 0x1001_0000);
    assert_eq!(FW_B_ADDR, 0x100D_0000);
}

#[test]
fn test_firmware_bank_size() {
    assert_eq!(FW_BANK_SIZE, 768 * 1024); // 768KB
}

#[test]
fn test_boot_data_address() {
    assert_eq!(BOOT_DATA_ADDR, 0x1019_0000);
}

#[test]
fn test_ram_update_constants() {
    assert_eq!(RAM_UPDATE_FLAG_ADDR, 0x2003_BFF0);
    assert_eq!(RAM_UPDATE_MAGIC, 0x0FDA_7E00);
}

#[test]
fn test_flash_sizes() {
    assert_eq!(FLASH_SECTOR_SIZE, 4096);
    assert_eq!(FLASH_PAGE_SIZE, 256);
}

#[test]
fn test_max_data_block_size() {
    assert_eq!(MAX_DATA_BLOCK_SIZE, 1024);
}

// --- Memory layout validation ---

#[test]
fn test_bank_a_does_not_overlap_bootloader() {
    // Bootloader is at FLASH_BASE, bank A should be after it
    assert!(FW_A_ADDR > FLASH_BASE);
}

#[test]
fn test_banks_do_not_overlap() {
    // Bank B should start after bank A ends
    let bank_a_end = FW_A_ADDR + FW_BANK_SIZE;
    assert!(FW_B_ADDR >= bank_a_end);
}

#[test]
fn test_boot_data_after_banks() {
    // Boot data should be after both firmware banks
    let bank_b_end = FW_B_ADDR + FW_BANK_SIZE;
    assert!(BOOT_DATA_ADDR >= bank_b_end);
}

// --- AckStatus tests ---

#[test]
fn test_ack_status_equality() {
    assert_eq!(AckStatus::Ok, AckStatus::Ok);
    assert_ne!(AckStatus::Ok, AckStatus::CrcError);
    assert_ne!(AckStatus::FlashError, AckStatus::BadCommand);
}

#[test]
fn test_ack_status_debug() {
    assert_eq!(format!("{:?}", AckStatus::Ok), "Ok");
    assert_eq!(format!("{:?}", AckStatus::CrcError), "CrcError");
    assert_eq!(format!("{:?}", AckStatus::FlashError), "FlashError");
    assert_eq!(format!("{:?}", AckStatus::BadCommand), "BadCommand");
    assert_eq!(format!("{:?}", AckStatus::BadState), "BadState");
    assert_eq!(format!("{:?}", AckStatus::BankInvalid), "BankInvalid");
}

// --- BootState tests ---

#[test]
fn test_boot_state_equality() {
    assert_eq!(BootState::Idle, BootState::Idle);
    assert_ne!(BootState::Idle, BootState::UpdateMode);
    assert_ne!(BootState::UpdateMode, BootState::Receiving);
}

#[test]
fn test_boot_state_debug() {
    assert_eq!(format!("{:?}", BootState::Idle), "Idle");
    assert_eq!(format!("{:?}", BootState::UpdateMode), "UpdateMode");
    assert_eq!(format!("{:?}", BootState::Receiving), "Receiving");
}

// --- Command tests ---

#[test]
fn test_command_get_status_debug() {
    let cmd = Command::GetStatus;
    assert!(format!("{:?}", cmd).contains("GetStatus"));
}

#[test]
fn test_command_start_update_debug() {
    let cmd = Command::StartUpdate {
        bank: 0,
        size: 1024,
        crc32: 0xDEADBEEF,
        version: 1,
    };
    let debug = format!("{:?}", cmd);
    assert!(debug.contains("StartUpdate"));
    assert!(debug.contains("1024"));
}

#[test]
fn test_command_data_block_debug() {
    let cmd = Command::DataBlock {
        offset: 0,
        data: vec![1, 2, 3, 4],
    };
    let debug = format!("{:?}", cmd);
    assert!(debug.contains("DataBlock"));
}

#[test]
fn test_command_finish_update_debug() {
    let cmd = Command::FinishUpdate;
    assert!(format!("{:?}", cmd).contains("FinishUpdate"));
}

#[test]
fn test_command_reboot_debug() {
    let cmd = Command::Reboot;
    assert!(format!("{:?}", cmd).contains("Reboot"));
}

#[test]
fn test_command_set_active_bank_debug() {
    let cmd = Command::SetActiveBank { bank: 1 };
    let debug = format!("{:?}", cmd);
    assert!(debug.contains("SetActiveBank"));
}

#[test]
fn test_command_wipe_all_debug() {
    let cmd = Command::WipeAll;
    assert!(format!("{:?}", cmd).contains("WipeAll"));
}

// --- Response tests ---

#[test]
fn test_response_ack_debug() {
    let resp = Response::Ack(AckStatus::Ok);
    let debug = format!("{:?}", resp);
    assert!(debug.contains("Ack"));
    assert!(debug.contains("Ok"));
}

#[test]
fn test_response_status_debug() {
    let resp = Response::Status {
        active_bank: 0,
        version_a: 1,
        version_b: 2,
        state: BootState::Idle,
    };
    let debug = format!("{:?}", resp);
    assert!(debug.contains("Status"));
    assert!(debug.contains("Idle"));
}

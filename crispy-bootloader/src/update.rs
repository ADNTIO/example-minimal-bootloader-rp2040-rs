// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Firmware update state machine over USB CDC.
//!
//! This module implements the update protocol:
//! - GetStatus: Query current bootloader state
//! - StartUpdate: Begin firmware upload to a bank
//! - DataBlock: Send firmware data chunks
//! - FinishUpdate: Verify CRC and commit the update
//! - Reboot: Restart the device

use crate::flash;
use crate::peripherals::{self, Peripherals};
use crate::usb_transport::UsbTransport;
use crispy_common::protocol::*;
use embedded_hal::digital::OutputPin;
use rp2040_hal as hal;
use usb_device::class_prelude::UsbBusAllocator;

/// Enter update mode: initialize USB and run the update loop.
pub fn enter_update_mode(p: &mut Peripherals) -> ! {
    defmt::println!("Update mode requested");

    crispy_common::blink(&mut p.led_pin, &mut p.timer, 10, 50);

    let mut usb = p.usb.take().expect("USB peripherals already taken");

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        usb.regs,
        usb.dpram,
        usb.clock,
        true,
        &mut usb.resets,
    ));

    peripherals::store_usb_bus(usb_bus);
    let mut transport = UsbTransport::new(peripherals::usb_bus_ref());

    defmt::println!("USB CDC initialized, entering update loop");
    p.led_pin.set_high().ok();

    run_update_mode(&mut transport)
}

/// Update state machine states.
enum UpdateState {
    /// Waiting for a new update to start.
    Idle,
    /// Actively receiving firmware data.
    Receiving {
        bank: u8,
        bank_addr: u32,
        expected_size: u32,
        expected_crc: u32,
        version: u32,
        bytes_received: u32,
    },
}

/// Run the update mode loop. Does not return (reboot via SCB::sys_reset).
pub fn run_update_mode(transport: &mut UsbTransport) -> ! {
    let mut state = UpdateState::Idle;

    loop {
        transport.poll();

        if let Some(cmd) = transport.try_receive() {
            state = handle_command(transport, state, cmd);
        }
    }
}

/// Dispatch a command to its handler.
fn handle_command(transport: &mut UsbTransport, state: UpdateState, cmd: Command) -> UpdateState {
    match cmd {
        Command::GetStatus => handle_get_status(transport, state),
        Command::StartUpdate {
            bank,
            size,
            crc32,
            version,
        } => handle_start_update(transport, state, bank, size, crc32, version),
        Command::DataBlock { offset, data } => handle_data_block(transport, state, offset, data),
        Command::FinishUpdate => handle_finish_update(transport, state),
        Command::Reboot => handle_reboot(transport),
        Command::SetActiveBank { bank } => handle_set_active_bank(transport, state, bank),
        Command::WipeAll => handle_wipe_all(transport, state),
    }
}

/// Handle GetStatus command: return current bootloader status.
fn handle_get_status(transport: &mut UsbTransport, state: UpdateState) -> UpdateState {
    let bd = flash::read_boot_data();
    let boot_state = match &state {
        UpdateState::Idle => BootState::UpdateMode,
        UpdateState::Receiving { .. } => BootState::Receiving,
    };
    transport.send(&Response::Status {
        active_bank: bd.active_bank,
        version_a: bd.version_a,
        version_b: bd.version_b,
        state: boot_state,
    });
    state
}

/// Handle StartUpdate command: validate parameters, erase bank, begin receiving.
fn handle_start_update(
    transport: &mut UsbTransport,
    state: UpdateState,
    bank: u8,
    size: u32,
    crc32: u32,
    version: u32,
) -> UpdateState {
    // Must be in Idle state
    if !matches!(state, UpdateState::Idle) {
        transport.send(&Response::Ack(AckStatus::BadState));
        return state;
    }

    // Validate bank number
    if bank > 1 {
        transport.send(&Response::Ack(AckStatus::BankInvalid));
        return state;
    }

    // Validate size
    if size == 0 || size > FW_BANK_SIZE {
        transport.send(&Response::Ack(AckStatus::BankInvalid));
        return state;
    }

    let bank_addr = if bank == 0 { FW_A_ADDR } else { FW_B_ADDR };

    // Erase the entire bank (rounded up to sector boundary)
    let erase_size = size.div_ceil(FLASH_SECTOR_SIZE) * FLASH_SECTOR_SIZE;
    let offset = flash::addr_to_offset(bank_addr);
    unsafe {
        flash::flash_erase(offset, erase_size);
    }

    transport.send(&Response::Ack(AckStatus::Ok));

    UpdateState::Receiving {
        bank,
        bank_addr,
        expected_size: size,
        expected_crc: crc32,
        version,
        bytes_received: 0,
    }
}

/// Handle DataBlock command: validate offset, program flash.
fn handle_data_block(
    transport: &mut UsbTransport,
    mut state: UpdateState,
    offset: u32,
    data: heapless::Vec<u8, MAX_DATA_BLOCK_SIZE>,
) -> UpdateState {
    let UpdateState::Receiving {
        bank_addr,
        ref mut bytes_received,
        expected_size,
        ..
    } = state
    else {
        transport.send(&Response::Ack(AckStatus::BadState));
        return state;
    };

    // Validate sequential offset
    if offset != *bytes_received {
        transport.send(&Response::Ack(AckStatus::BadCommand));
        return state;
    }

    // Validate data doesn't exceed expected size
    let data_len = data.len() as u32;
    if *bytes_received + data_len > expected_size {
        transport.send(&Response::Ack(AckStatus::BadCommand));
        return state;
    }

    // Pad data to 256-byte page boundary for flash programming
    let mut page_buf = [0xFFu8; MAX_DATA_BLOCK_SIZE + FLASH_PAGE_SIZE as usize];
    let actual_len = data.len();
    page_buf[..actual_len].copy_from_slice(&data);
    let padded_len = actual_len.div_ceil(FLASH_PAGE_SIZE as usize) * FLASH_PAGE_SIZE as usize;

    let flash_offset = flash::addr_to_offset(bank_addr) + *bytes_received;
    unsafe {
        flash::flash_program(flash_offset, page_buf.as_ptr(), padded_len);
    }

    *bytes_received += data_len;
    transport.send(&Response::Ack(AckStatus::Ok));
    state
}

/// Handle FinishUpdate command: verify CRC, update BootData.
fn handle_finish_update(transport: &mut UsbTransport, state: UpdateState) -> UpdateState {
    let UpdateState::Receiving {
        bank,
        bank_addr,
        expected_size,
        expected_crc,
        version,
        bytes_received,
    } = state
    else {
        transport.send(&Response::Ack(AckStatus::BadState));
        return state;
    };

    // Verify all data was received
    if bytes_received != expected_size {
        transport.send(&Response::Ack(AckStatus::BadCommand));
        return UpdateState::Receiving {
            bank,
            bank_addr,
            expected_size,
            expected_crc,
            version,
            bytes_received,
        };
    }

    // Verify CRC
    let actual_crc = flash::compute_crc32(bank_addr, expected_size);
    if actual_crc != expected_crc {
        defmt::println!(
            "CRC mismatch: expected 0x{:08x}, got 0x{:08x}",
            expected_crc,
            actual_crc
        );
        transport.send(&Response::Ack(AckStatus::CrcError));
        return UpdateState::Idle;
    }

    // Update BootData
    let mut bd = flash::read_boot_data();
    bd.active_bank = bank;
    bd.confirmed = 0; // unconfirmed until firmware confirms
    bd.boot_attempts = 0;

    if bank == 0 {
        bd.version_a = version;
        bd.crc_a = expected_crc;
        bd.size_a = expected_size;
    } else {
        bd.version_b = version;
        bd.crc_b = expected_crc;
        bd.size_b = expected_size;
    }

    unsafe {
        flash::write_boot_data(&bd);
    }

    transport.send(&Response::Ack(AckStatus::Ok));
    UpdateState::Idle
}

/// Handle Reboot command: send ACK and reset the system.
fn handle_reboot(transport: &mut UsbTransport) -> ! {
    transport.send(&Response::Ack(AckStatus::Ok));
    // Small delay to let the ACK be sent
    cortex_m::asm::delay(12_000_000); // ~1s at 12MHz
    cortex_m::peripheral::SCB::sys_reset();
}

/// Handle SetActiveBank command: change the active bank for next boot.
fn handle_set_active_bank(
    transport: &mut UsbTransport,
    state: UpdateState,
    bank: u8,
) -> UpdateState {
    // Must be in Idle state
    if !matches!(state, UpdateState::Idle) {
        transport.send(&Response::Ack(AckStatus::BadState));
        return state;
    }

    // Validate bank number
    if bank > 1 {
        transport.send(&Response::Ack(AckStatus::BankInvalid));
        return state;
    }

    // Read current BootData and update active bank
    let mut bd = flash::read_boot_data();

    // Check that the target bank has valid firmware
    let (size, crc) = if bank == 0 {
        (bd.size_a, bd.crc_a)
    } else {
        (bd.size_b, bd.crc_b)
    };

    if size == 0 {
        defmt::println!("SetActiveBank: bank {} has no firmware", bank);
        transport.send(&Response::Ack(AckStatus::BankInvalid));
        return state;
    }

    // Verify CRC of the target bank
    let bank_addr = if bank == 0 { FW_A_ADDR } else { FW_B_ADDR };
    let actual_crc = flash::compute_crc32(bank_addr, size);
    if actual_crc != crc {
        defmt::println!(
            "SetActiveBank: bank {} CRC mismatch (expected 0x{:08x}, got 0x{:08x})",
            bank,
            crc,
            actual_crc
        );
        transport.send(&Response::Ack(AckStatus::CrcError));
        return state;
    }

    // Update BootData
    bd.active_bank = bank;
    bd.confirmed = 0; // unconfirmed until firmware confirms
    bd.boot_attempts = 0;

    unsafe {
        flash::write_boot_data(&bd);
    }

    defmt::println!("SetActiveBank: switched to bank {}", bank);
    transport.send(&Response::Ack(AckStatus::Ok));
    state
}

fn handle_wipe_all(transport: &mut UsbTransport, state: UpdateState) -> UpdateState {
    if !matches!(state, UpdateState::Idle) {
        transport.send(&Response::Ack(AckStatus::BadState));
        return state;
    }

    defmt::println!("Resetting boot data");
    unsafe {
        flash::write_boot_data(&BootData::default_new());
    }

    transport.send(&Response::Ack(AckStatus::Ok));
    state
}

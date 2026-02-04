// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Command implementations for bootloader operations.

use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{bail, Context, Result};
use crc::{Crc, CRC_32_ISO_HDLC};
use indicatif::{ProgressBar, ProgressStyle};

use crispy_common::protocol::{AckStatus, Command, Response};
use crispy_common::MAX_DATA_BLOCK_SIZE;

use crate::transport::Transport;

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
const CHUNK_SIZE: usize = MAX_DATA_BLOCK_SIZE;

/// Get and display bootloader status.
pub fn status(transport: &mut Transport) -> Result<()> {
    let response = transport.send_recv(&Command::GetStatus)?;

    match response {
        Response::Status {
            active_bank,
            version_a,
            version_b,
            state,
        } => {
            println!("Bootloader Status:");
            println!(
                "  Active bank: {} ({})",
                active_bank,
                if active_bank == 0 { "A" } else { "B" }
            );
            println!("  Version A:   {}", version_a);
            println!("  Version B:   {}", version_b);
            println!("  State:       {:?}", state);
        }
        Response::Ack(status) => {
            println!("Unexpected ACK response: {:?}", status);
        }
    }

    Ok(())
}

/// Upload firmware to the specified bank.
pub fn upload(transport: &mut Transport, file: &Path, bank: u8, version: u32) -> Result<()> {
    // Read firmware file
    let firmware = fs::read(file).with_context(|| format!("Failed to read {}", file.display()))?;
    let size = firmware.len() as u32;
    let crc32 = CRC32.checksum(&firmware);

    println!(
        "Firmware: {} ({} bytes, CRC32: 0x{:08x})",
        file.display(),
        size,
        crc32
    );
    println!(
        "Target:   Bank {} ({})",
        bank,
        if bank == 0 { "A" } else { "B" }
    );
    println!("Version:  {}", version);
    println!();

    // Start update (includes erasing the target bank - can take 30+ seconds)
    print!("Starting update (erasing bank)... ");
    std::io::stdout().flush()?;

    let response = transport.send_recv_timeout(
        &Command::StartUpdate {
            bank,
            size,
            crc32,
            version,
        },
        60_000, // 60 second timeout for bank erase
    )?;

    match response {
        Response::Ack(AckStatus::Ok) => println!("OK"),
        Response::Ack(status) => bail!("StartUpdate failed: {:?}", status),
        _ => bail!("Unexpected response: {:?}", response),
    }

    // Send data blocks
    let pb = ProgressBar::new(size as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            )?
            .progress_chars("#>-"),
    );

    let mut offset = 0u32;
    for chunk in firmware.chunks(CHUNK_SIZE) {
        let response = transport.send_recv(&Command::DataBlock {
            offset,
            data: chunk.to_vec(),
        })?;

        match response {
            Response::Ack(AckStatus::Ok) => {}
            Response::Ack(status) => {
                pb.abandon();
                bail!("DataBlock failed at offset {}: {:?}", offset, status);
            }
            _ => {
                pb.abandon();
                bail!("Unexpected response at offset {}: {:?}", offset, response);
            }
        }

        offset += chunk.len() as u32;
        pb.set_position(offset as u64);
    }

    pb.finish_with_message("Upload complete");
    println!();

    // Finish update
    print!("Finalizing... ");
    std::io::stdout().flush()?;

    let response = transport.send_recv(&Command::FinishUpdate)?;

    match response {
        Response::Ack(AckStatus::Ok) => println!("OK"),
        Response::Ack(AckStatus::CrcError) => bail!("CRC verification failed!"),
        Response::Ack(status) => bail!("FinishUpdate failed: {:?}", status),
        _ => bail!("Unexpected response: {:?}", response),
    }

    println!();
    println!("Firmware uploaded successfully!");
    println!(
        "Use 'crispy-upload --port {} reboot' to restart the device.",
        transport.port_name()
    );

    Ok(())
}

/// Set the active bank for the next boot.
pub fn set_bank(transport: &mut Transport, bank: u8) -> Result<()> {
    println!(
        "Setting active bank to {} ({})...",
        bank,
        if bank == 0 { "A" } else { "B" }
    );

    let response = transport.send_recv(&Command::SetActiveBank { bank })?;

    match response {
        Response::Ack(AckStatus::Ok) => {
            println!("Active bank set successfully.");
            println!(
                "Use 'crispy-upload --port {} reboot' to restart the device.",
                transport.port_name()
            );
        }
        Response::Ack(AckStatus::BankInvalid) => bail!("Invalid bank: must be 0 (A) or 1 (B)"),
        Response::Ack(AckStatus::CrcError) => {
            bail!("Bank {} has no valid firmware (CRC check failed)", bank)
        }
        Response::Ack(status) => bail!("SetActiveBank failed: {:?}", status),
        _ => bail!("Unexpected response: {:?}", response),
    }

    Ok(())
}

/// Wipe all firmware banks and reset boot data.
pub fn wipe(transport: &mut Transport) -> Result<()> {
    println!("Resetting boot data (invalidates all firmware)...");

    let response = transport.send_recv(&Command::WipeAll)?;

    match response {
        Response::Ack(AckStatus::Ok) => {
            println!("Boot data reset. Firmware banks marked as invalid.");
            println!("Device is now in update mode, ready for firmware upload.");
        }
        Response::Ack(AckStatus::BadState) => {
            bail!("Cannot wipe: device is not in idle state (upload in progress?)")
        }
        Response::Ack(status) => bail!("Wipe failed: {:?}", status),
        _ => bail!("Unexpected response: {:?}", response),
    }

    Ok(())
}

/// Reboot the device.
pub fn reboot(transport: &mut Transport) -> Result<()> {
    print!("Rebooting device... ");
    std::io::stdout().flush()?;

    let response = transport.send_recv(&Command::Reboot)?;

    match response {
        Response::Ack(AckStatus::Ok) => println!("OK"),
        Response::Ack(status) => bail!("Reboot failed: {:?}", status),
        _ => bail!("Unexpected response: {:?}", response),
    }

    Ok(())
}

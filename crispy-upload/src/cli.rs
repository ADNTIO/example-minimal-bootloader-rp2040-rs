// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Command-line interface definitions.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands;
use crate::transport::Transport;

/// Command-line arguments.
#[derive(Parser)]
#[command(name = "crispy-upload")]
#[command(about = "Firmware upload tool for crispy-bootloader")]
pub struct Cli {
    /// Serial port (e.g., /dev/ttyACM0)
    #[arg(short, long)]
    pub port: String,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Get bootloader status
    Status,

    /// Upload firmware to a bank
    Upload {
        /// Firmware binary file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Target bank (0 = A, 1 = B)
        #[arg(short, long, default_value = "0")]
        bank: u8,

        /// Firmware version number
        #[arg(short, long, default_value = "1")]
        version: u32,
    },

    /// Set the active bank for the next boot (without uploading new firmware)
    SetBank {
        /// Target bank (0 = A, 1 = B)
        #[arg(value_name = "BANK")]
        bank: u8,
    },

    /// Wipe all firmware banks and reset boot data
    Wipe,

    /// Reboot the device
    Reboot,
}

/// Execute the parsed CLI command.
pub fn run(cli: Cli) -> Result<()> {
    let mut transport = Transport::new(&cli.port)?;

    match cli.command {
        Commands::Status => commands::status(&mut transport),
        Commands::Upload {
            file,
            bank,
            version,
        } => commands::upload(&mut transport, &file, bank, version),
        Commands::SetBank { bank } => commands::set_bank(&mut transport, bank),
        Commands::Wipe => commands::wipe(&mut transport),
        Commands::Reboot => commands::reboot(&mut transport),
    }
}

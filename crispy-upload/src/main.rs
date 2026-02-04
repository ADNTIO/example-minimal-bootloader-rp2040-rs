// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Firmware upload tool for crispy-bootloader via USB CDC.
//!
//! Usage:
//!   crispy-upload --port /dev/ttyACM0 status
//!   crispy-upload --port /dev/ttyACM0 upload firmware.bin --bank 0 --version 1
//!   crispy-upload --port /dev/ttyACM0 reboot

mod cli;
mod commands;
mod transport;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    cli::run(args)
}

// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Common types and utilities for crispy-bootloader.
//!
//! This crate supports both `no_std` (embedded) and `std` (host) environments:
//! - Default: `no_std` mode for embedded targets
//! - `std` feature: Enables `std` support for host tools
//! - `embedded` feature: Enables embedded-specific board support (rp2040-hal)

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate alloc;

pub mod cobs;
pub mod protocol;

// Flash operations for firmware (requires embedded feature)
#[cfg(feature = "embedded")]
pub mod flash;

// Re-export commonly used types
pub use protocol::{AckStatus, BootData, BootState, Command, Response};
pub use protocol::{BOOT_DATA_ADDR, BOOT_DATA_MAGIC, FLASH_BASE, FW_A_ADDR, FW_B_ADDR};
pub use protocol::{FLASH_PAGE_SIZE, FLASH_SECTOR_SIZE, FW_BANK_SIZE, MAX_DATA_BLOCK_SIZE};

// Embedded-specific exports (only with embedded feature)
#[cfg(feature = "embedded")]
use embedded_hal::delay::DelayNs;
#[cfg(feature = "embedded")]
use embedded_hal::digital::OutputPin;
#[cfg(feature = "embedded")]
use rp2040_hal as hal;

#[cfg(feature = "embedded")]
pub type LedPin =
    hal::gpio::Pin<hal::gpio::bank0::Gpio25, hal::gpio::FunctionSioOutput, hal::gpio::PullDown>;

/// Initialize RP2040 board peripherals.
///
/// # Safety
/// Uses `Peripherals::steal()` — caller must ensure exclusive peripheral access.
#[cfg(feature = "embedded")]
pub fn init_board() -> (hal::Timer, LedPin) {
    let mut pac = unsafe { hal::pac::Peripherals::steal() };

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    (timer, pins.gpio25.into_push_pull_output())
}

/// Blink an LED a specified number of times.
#[cfg(feature = "embedded")]
pub fn blink(led: &mut impl OutputPin, timer: &mut impl DelayNs, count: u32, period_ms: u32) {
    for _ in 0..count {
        led.set_high().ok();
        timer.delay_ms(period_ms);
        led.set_low().ok();
        timer.delay_ms(period_ms);
    }
}

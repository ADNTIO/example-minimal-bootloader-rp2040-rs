// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Crispy Bootloader for RP2040 with A/B multiboot and USB CDC update mode.

#![no_std]
#![no_main]

mod boot;
mod flash;
mod peripherals;
mod update;
mod usb_transport;

use defmt_rtt as _;
use embedded_hal::digital::InputPin;
use panic_probe as _;

defmt::timestamp!("{=u64:us}", { 0 });

use cortex_m_rt::entry;

#[unsafe(link_section = ".boot2")]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

#[entry]
fn main() -> ! {
    defmt::println!("Bootloader init");

    let mut p = peripherals::init();

    crispy_common::blink(&mut p.led_pin, &mut p.timer, 3, 200);
    flash::init();

    let gp2_low = p.gp2.is_low().unwrap_or(false);
    if boot::check_update_trigger(gp2_low) {
        update::enter_update_mode(&mut p);
    }

    boot::run_normal_boot(&mut p);
}

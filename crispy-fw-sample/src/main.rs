// Copyright (c) 2026 ADNT Sarl <info@adnt.io>
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]

use crispy_common::protocol::{
    BootData, BOOT_DATA_ADDR, FLASH_BASE, FLASH_PAGE_SIZE, FLASH_SECTOR_SIZE,
    RAM_UPDATE_FLAG_ADDR, RAM_UPDATE_MAGIC,
};
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use embedded_hal::digital::StatefulOutputPin;
use panic_probe as _;
use rp2040_hal as hal;
use rp2040_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

defmt::timestamp!("{=u64:us}", { 0 });

use cortex_m_rt::entry;

/// Static storage for UsbBusAllocator (required by usb-device for 'static lifetime).
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

fn usb_bus_ref() -> &'static UsbBusAllocator<UsbBus> {
    unsafe { (*core::ptr::addr_of!(USB_BUS)).as_ref().unwrap() }
}

/// Confirm the current boot to the bootloader by writing confirmed=1
/// and resetting boot_attempts=0 in BootData.
fn confirm_boot() {
    let bd = unsafe { BootData::read_from(BOOT_DATA_ADDR) };
    if !bd.is_valid() {
        defmt::println!("BootData invalid, skipping confirmation");
        return;
    }
    if bd.confirmed == 1 {
        defmt::println!("Boot already confirmed");
        return;
    }

    defmt::println!("Confirming boot (bank={})", bd.active_bank);

    let mut bd = bd;
    bd.confirmed = 1;
    bd.boot_attempts = 0;

    let offset = BOOT_DATA_ADDR - FLASH_BASE;

    // Pad to 256-byte page
    let mut page = [0xFFu8; FLASH_PAGE_SIZE as usize];
    let src = bd.as_bytes();
    page[..src.len()].copy_from_slice(src);

    unsafe {
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
        rp2040_hal::rom_data::flash_range_program(offset, page.as_ptr(), page.len());
        rp2040_hal::rom_data::flash_flush_cache();
        rp2040_hal::rom_data::flash_enter_cmd_xip();
        cortex_m::interrupt::enable();
    }

    defmt::println!("Boot confirmed successfully");
}

/// Reboot into bootloader update mode.
/// Writes the RAM magic flag and triggers a system reset.
/// The bootloader will detect the flag and enter update mode.
fn reboot_to_bootloader() -> ! {
    defmt::println!("Rebooting to bootloader update mode...");

    // Write magic value to RAM flag address
    unsafe {
        (RAM_UPDATE_FLAG_ADDR as *mut u32).write_volatile(RAM_UPDATE_MAGIC);
    }

    // Small delay to allow defmt to flush
    cortex_m::asm::delay(1_000_000);

    // Trigger system reset
    cortex_m::peripheral::SCB::sys_reset();
}

/// Process a received command line and return a response.
/// Returns true if we should reboot to bootloader.
fn process_command(line: &str, serial: &mut SerialPort<UsbBus>) -> bool {
    let line = line.trim();

    match line {
        "help" | "?" => {
            let _ = serial.write(b"Available commands:\r\n");
            let _ = serial.write(b"  help     - Show this help\r\n");
            let _ = serial.write(b"  status   - Show boot status\r\n");
            let _ = serial.write(b"  bootload - Reboot to bootloader update mode\r\n");
            let _ = serial.write(b"  reboot   - Reboot normally\r\n");
        }
        "status" => {
            let bd = unsafe { BootData::read_from(BOOT_DATA_ADDR) };
            if bd.is_valid() {
                let mut buf = [0u8; 256];
                let len = format_status(&bd, &mut buf);
                let _ = serial.write(&buf[..len]);
            } else {
                let _ = serial.write(b"BootData: invalid\r\n");
            }
        }
        "bootload" => {
            let _ = serial.write(b"Rebooting to bootloader...\r\n");
            return true;
        }
        "reboot" => {
            let _ = serial.write(b"Rebooting...\r\n");
            cortex_m::asm::delay(1_000_000);
            cortex_m::peripheral::SCB::sys_reset();
        }
        "" => {}
        _ => {
            let _ = serial.write(b"Unknown command. Type 'help' for available commands.\r\n");
        }
    }

    false
}

fn format_status(bd: &BootData, buf: &mut [u8]) -> usize {
    use core::fmt::Write;

    struct BufWriter<'b> {
        buf: &'b mut [u8],
        pos: usize,
    }

    impl<'b> Write for BufWriter<'b> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - self.pos;
            let to_write = bytes.len().min(remaining);
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
            Ok(())
        }
    }

    let mut writer = BufWriter { buf, pos: 0 };
    let _ = write!(
        writer,
        "Boot status:\r\n  Bank: {} ({})\r\n  Confirmed: {}\r\n  Attempts: {}\r\n  Version A: {}\r\n  Version B: {}\r\n",
        bd.active_bank,
        if bd.active_bank == 0 { "A" } else { "B" },
        bd.confirmed,
        bd.boot_attempts,
        bd.version_a,
        bd.version_b
    );

    writer.pos
}

#[entry]
fn main() -> ! {
    defmt::println!("Firmware started!");

    // --- Inline peripheral init (need USB access) ---
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

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output();

    // Blink to signal firmware alive
    crispy_common::blink(&mut led_pin, &mut timer, 5, 100);

    confirm_boot();

    // Initialize USB
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    unsafe {
        USB_BUS = Some(usb_bus);
    }

    let mut serial = SerialPort::new(usb_bus_ref());
    let mut usb_dev = UsbDeviceBuilder::new(usb_bus_ref(), UsbVidPid(0x2E8A, 0x000B))
        .strings(&[StringDescriptors::default()
            .manufacturer("ADNT")
            .product("Crispy Firmware")
            .serial_number("FW001")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    defmt::println!("USB CDC initialized, entering main loop");
    defmt::println!("Connect via serial terminal and type 'help' for commands");

    let mut cmd_buf = [0u8; 64];
    let mut cmd_pos = 0usize;
    let mut blink_counter = 0u32;

    loop {
        // Poll USB
        usb_dev.poll(&mut [&mut serial]);

        // Read incoming data
        let mut buf = [0u8; 64];
        if let Ok(count) = serial.read(&mut buf) {
            for &byte in &buf[..count] {
                // Echo character
                let _ = serial.write(&[byte]);

                if byte == b'\r' || byte == b'\n' {
                    let _ = serial.write(b"\r\n");

                    if cmd_pos > 0 {
                        if let Ok(line) = core::str::from_utf8(&cmd_buf[..cmd_pos]) {
                            if process_command(line, &mut serial) {
                                // Flush USB before rebooting
                                for _ in 0..100 {
                                    usb_dev.poll(&mut [&mut serial]);
                                    cortex_m::asm::delay(10_000);
                                }
                                reboot_to_bootloader();
                            }
                        }
                        cmd_pos = 0;
                    }
                } else if byte == 0x7F || byte == 0x08 {
                    // Backspace
                    if cmd_pos > 0 {
                        cmd_pos -= 1;
                        let _ = serial.write(b"\x08 \x08");
                    }
                } else if cmd_pos < cmd_buf.len() {
                    cmd_buf[cmd_pos] = byte;
                    cmd_pos += 1;
                }
            }
        }

        // Slow blink LED to show activity
        blink_counter += 1;
        if blink_counter >= 500_000 {
            blink_counter = 0;
            if led_pin.is_set_high().unwrap_or(false) {
                led_pin.set_low().ok();
            } else {
                led_pin.set_high().ok();
            }
        }
    }
}

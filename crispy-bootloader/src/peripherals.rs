// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Peripheral initialization for the bootloader.

use rp2040_hal as hal;
use rp2040_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;

pub type LedPin =
    hal::gpio::Pin<hal::gpio::bank0::Gpio25, hal::gpio::FunctionSioOutput, hal::gpio::PullDown>;
pub type Gp2Pin =
    hal::gpio::Pin<hal::gpio::bank0::Gpio2, hal::gpio::FunctionSioInput, hal::gpio::PullUp>;

/// Static storage for UsbBusAllocator (required by usb-device for 'static lifetime).
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

pub fn usb_bus_ref() -> &'static UsbBusAllocator<UsbBus> {
    unsafe { (*core::ptr::addr_of!(USB_BUS)).as_ref().unwrap() }
}

pub fn store_usb_bus(bus: UsbBusAllocator<UsbBus>) {
    unsafe {
        USB_BUS = Some(bus);
    }
}

pub struct Peripherals {
    pub led_pin: LedPin,
    pub gp2: Gp2Pin,
    pub timer: hal::Timer,
    pub usb: Option<UsbPeripherals>,
}

pub struct UsbPeripherals {
    pub regs: hal::pac::USBCTRL_REGS,
    pub dpram: hal::pac::USBCTRL_DPRAM,
    pub clock: hal::clocks::UsbClock,
    pub resets: hal::pac::RESETS,
}

pub fn init() -> Peripherals {
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

    Peripherals {
        led_pin: pins.gpio25.into_push_pull_output(),
        gp2: pins.gpio2.into_pull_up_input(),
        timer,
        usb: Some(UsbPeripherals {
            regs: pac.USBCTRL_REGS,
            dpram: pac.USBCTRL_DPRAM,
            clock: clocks.usb_clock,
            resets: pac.RESETS,
        }),
    }
}

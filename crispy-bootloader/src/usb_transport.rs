// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! USB CDC transport with COBS-framed postcard serialization.

use crispy_common::protocol::{Command, Response};
use rp2040_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

const RX_BUF_SIZE: usize = 2048;
const TX_BUF_SIZE: usize = 2048;

pub struct UsbTransport {
    serial: SerialPort<'static, UsbBus>,
    usb_dev: UsbDevice<'static, UsbBus>,
    rx_buf: [u8; RX_BUF_SIZE],
    rx_pos: usize,
}

impl UsbTransport {
    pub fn new(usb_bus: &'static UsbBusAllocator<UsbBus>) -> Self {
        let serial = SerialPort::new(usb_bus);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2E8A, 0x000A))
            .strings(&[StringDescriptors::default()
                .manufacturer("ADNT")
                .product("Crispy Bootloader")
                .serial_number("0001")])
            .unwrap()
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        Self {
            serial,
            usb_dev,
            rx_buf: [0u8; RX_BUF_SIZE],
            rx_pos: 0,
        }
    }

    /// Poll USB device. Must be called frequently.
    pub fn poll(&mut self) -> bool {
        self.usb_dev.poll(&mut [&mut self.serial])
    }

    /// Try to receive a complete COBS-framed command.
    /// Returns `Some(Command)` when a full frame has been decoded.
    pub fn try_receive(&mut self) -> Option<Command> {
        // Read available bytes from USB serial
        let mut tmp = [0u8; 64];
        match self.serial.read(&mut tmp) {
            Ok(count) if count > 0 => {
                for &byte in &tmp[..count] {
                    if byte == 0x00 {
                        // COBS delimiter — try to decode the accumulated frame
                        if self.rx_pos > 0 {
                            let result = postcard::from_bytes_cobs::<Command>(
                                &mut self.rx_buf[..self.rx_pos],
                            );
                            self.rx_pos = 0;
                            return result.ok();
                        }
                    } else if self.rx_pos < RX_BUF_SIZE {
                        self.rx_buf[self.rx_pos] = byte;
                        self.rx_pos += 1;
                    } else {
                        // Overflow — discard frame
                        self.rx_pos = 0;
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Send a response as a COBS-framed postcard message.
    pub fn send(&mut self, resp: &Response) {
        let mut buf = [0u8; TX_BUF_SIZE];
        if let Ok(encoded) = postcard::to_slice_cobs(resp, &mut buf) {
            let mut offset = 0;
            while offset < encoded.len() {
                match self.serial.write(&encoded[offset..]) {
                    Ok(n) => offset += n,
                    Err(UsbError::WouldBlock) => {
                        self.poll();
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

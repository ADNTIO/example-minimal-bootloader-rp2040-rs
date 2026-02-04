// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Serial transport layer for bootloader communication.

use anyhow::{bail, Context, Result};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;

use crispy_common::protocol::{Command, Response};

/// Default timeout for serial operations in milliseconds.
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// USB CDC transport for communicating with the bootloader.
pub struct Transport {
    port: Box<dyn SerialPort>,
    rx_buf: Vec<u8>,
}

impl Transport {
    /// Create a new transport connection to the specified serial port.
    pub fn new(port_name: &str) -> Result<Self> {
        Self::with_timeout(port_name, DEFAULT_TIMEOUT_MS)
    }

    /// Create a new transport connection with a custom timeout.
    pub fn with_timeout(port_name: &str, timeout_ms: u64) -> Result<Self> {
        let port = serialport::new(port_name, 115200)
            .timeout(Duration::from_millis(timeout_ms))
            .open()
            .with_context(|| format!("Failed to open serial port {}", port_name))?;

        Ok(Self {
            port,
            rx_buf: Vec::with_capacity(4096),
        })
    }

    /// Get the port name.
    pub fn port_name(&self) -> String {
        self.port.name().unwrap_or_else(|| "?".to_string())
    }

    /// Send a command to the bootloader.
    pub fn send(&mut self, cmd: &Command) -> Result<()> {
        let mut buf = [0u8; 2048];
        let encoded = postcard::to_slice_cobs(cmd, &mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to serialize command: {}", e))?;
        self.port
            .write_all(encoded)
            .map_err(|e| anyhow::anyhow!("Failed to write to serial port: {}", e))?;
        self.port.flush()?;
        Ok(())
    }

    /// Receive a response from the bootloader.
    pub fn receive(&mut self) -> Result<Response> {
        self.rx_buf.clear();
        let mut byte = [0u8; 1];

        // Read until we get delimiter (0x00)
        loop {
            match self.port.read(&mut byte) {
                Ok(1) => {
                    self.rx_buf.push(byte[0]);
                    if byte[0] == 0 {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    bail!("Timeout waiting for response");
                }
                Err(e) => bail!("Serial read error: {}", e),
            }
        }

        // Use postcard's COBS decoder for consistency with bootloader
        postcard::from_bytes_cobs(&mut self.rx_buf).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize response: {} (raw {} bytes: {:02x?})",
                e,
                self.rx_buf.len(),
                &self.rx_buf[..self.rx_buf.len().min(32)]
            )
        })
    }

    fn drain_rx(&mut self) {
        let mut buf = [0u8; 64];
        let old_timeout = self.port.timeout();
        let _ = self.port.set_timeout(Duration::from_millis(10));
        while self.port.read(&mut buf).unwrap_or(0) > 0 {}
        let _ = self.port.set_timeout(old_timeout);
    }

    /// Send a command and wait for the response.
    pub fn send_recv(&mut self, cmd: &Command) -> Result<Response> {
        self.drain_rx();
        self.send(cmd)?;
        self.receive()
    }

    /// Send a command and wait for the response with a custom timeout.
    pub fn send_recv_timeout(&mut self, cmd: &Command, timeout_ms: u64) -> Result<Response> {
        // Save current timeout
        let old_timeout = self.port.timeout();

        // Set new timeout
        self.port
            .set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|e| anyhow::anyhow!("Failed to set timeout: {}", e))?;

        // Send and receive
        let result = self.send_recv(cmd);

        // Restore old timeout
        let _ = self.port.set_timeout(old_timeout);

        result
    }
}

# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
Transport layer for crispy bootloader communication.

Handles serial port communication with COBS framing.
"""

import time
from pathlib import Path
from typing import Callable, Optional

import serial

from .crc32 import crc32
from .protocol import (
    ResponseType,
    AckResponse,
    StatusResponse,
    AckStatus,
    decode_response,
    encode_get_status,
    encode_start_update,
    encode_data_block,
    encode_finish_update,
    encode_reboot,
)


class TransportError(Exception):
    """Base exception for transport errors."""
    pass


class TimeoutError(TransportError):
    """Timeout waiting for response."""
    pass


class ProtocolError(TransportError):
    """Protocol-level error (unexpected response, etc.)."""
    pass


class UploadError(TransportError):
    """Error during firmware upload."""
    pass


class Transport:
    """
    USB CDC transport for crispy bootloader.

    Can be used as a context manager:
        with Transport("/dev/ttyACM0") as t:
            status = t.get_status()
    """

    def __init__(
        self,
        port: str,
        baudrate: int = 115200,
        timeout: float = 5.0,
    ):
        """
        Open a connection to the bootloader.

        Args:
            port: Serial port path (e.g., "/dev/ttyACM0")
            baudrate: Baud rate (default 115200)
            timeout: Read timeout in seconds (default 5.0)
        """
        self._ser = serial.Serial(port, baudrate, timeout=timeout)
        time.sleep(0.1)  # Let the device settle

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False

    def close(self):
        """Close the serial connection."""
        if self._ser and self._ser.is_open:
            self._ser.close()

    @property
    def port(self) -> str:
        """Return the serial port name."""
        return self._ser.port

    def _send(self, data: bytes):
        """Send raw bytes."""
        self._ser.write(data)
        self._ser.flush()

    def _receive(self) -> bytes:
        """Receive bytes until 0x00 delimiter."""
        result = bytearray()
        while True:
            byte = self._ser.read(1)
            if not byte:
                raise TimeoutError("Timeout waiting for response")
            result.append(byte[0])
            if byte[0] == 0:
                break
        return bytes(result)

    def _send_recv(self, data: bytes) -> ResponseType:
        """Send data and receive response."""
        self._send(data)
        return decode_response(self._receive())

    def send(self, data: bytes) -> None:
        """Send a pre-encoded command."""
        self._send(data)

    def receive(self) -> ResponseType:
        """Receive and decode a response."""
        return decode_response(self._receive())

    def get_status(self) -> StatusResponse:
        """
        Get bootloader status.

        Returns:
            StatusResponse with active_bank, versions, and state

        Raises:
            ProtocolError: If response is not a StatusResponse
        """
        resp = self._send_recv(encode_get_status())
        if not isinstance(resp, StatusResponse):
            raise ProtocolError(f"Expected StatusResponse, got {type(resp).__name__}")
        return resp

    def start_update(self, bank: int, size: int, crc: int, version: int) -> AckResponse:
        """
        Start a firmware update.

        Args:
            bank: Target bank (0=A, 1=B)
            size: Firmware size in bytes
            crc: CRC-32 checksum of firmware
            version: Firmware version number

        Returns:
            AckResponse
        """
        resp = self._send_recv(encode_start_update(bank, size, crc, version))
        if not isinstance(resp, AckResponse):
            raise ProtocolError(f"Expected AckResponse, got {type(resp).__name__}")
        return resp

    def send_data_block(self, offset: int, data: bytes) -> AckResponse:
        """
        Send a data block.

        Args:
            offset: Byte offset in firmware
            data: Data chunk (max 1024 bytes)

        Returns:
            AckResponse
        """
        resp = self._send_recv(encode_data_block(offset, data))
        if not isinstance(resp, AckResponse):
            raise ProtocolError(f"Expected AckResponse, got {type(resp).__name__}")
        return resp

    def finish_update(self) -> AckResponse:
        """
        Finish the firmware update.

        The bootloader will verify CRC and update boot data.

        Returns:
            AckResponse
        """
        resp = self._send_recv(encode_finish_update())
        if not isinstance(resp, AckResponse):
            raise ProtocolError(f"Expected AckResponse, got {type(resp).__name__}")
        return resp

    def reboot(self) -> AckResponse:
        """
        Reboot the device.

        Returns:
            AckResponse
        """
        resp = self._send_recv(encode_reboot())
        if not isinstance(resp, AckResponse):
            raise ProtocolError(f"Expected AckResponse, got {type(resp).__name__}")
        return resp

    def upload_firmware(
        self,
        firmware: bytes,
        bank: int,
        version: int,
        chunk_size: int = 1024,
        progress_callback: Optional[Callable[[int, int], None]] = None,
    ) -> None:
        """
        Upload firmware to a bank.

        Args:
            firmware: Firmware binary data
            bank: Target bank (0=A, 1=B)
            version: Firmware version number
            chunk_size: Size of data chunks (default 1024)
            progress_callback: Optional callback(bytes_sent, total_bytes)

        Raises:
            UploadError: If upload fails
        """
        size = len(firmware)
        checksum = crc32(firmware)

        # Start update
        resp = self.start_update(bank, size, checksum, version)
        if not resp.is_ok:
            raise UploadError(f"StartUpdate failed: {resp.status}")

        # Send data blocks
        offset = 0
        while offset < size:
            chunk = firmware[offset:offset + chunk_size]
            resp = self.send_data_block(offset, chunk)

            if not resp.is_ok:
                raise UploadError(f"DataBlock failed at offset {offset}: {resp.status}")

            offset += len(chunk)

            if progress_callback:
                progress_callback(offset, size)

        # Finish update
        resp = self.finish_update()
        if not resp.is_ok:
            if resp.status == AckStatus.CRC_ERROR:
                raise UploadError("CRC verification failed")
            raise UploadError(f"FinishUpdate failed: {resp.status}")

    def upload_firmware_file(
        self,
        path: Path,
        bank: int,
        version: int,
        chunk_size: int = 1024,
        progress_callback: Optional[Callable[[int, int], None]] = None,
    ) -> int:
        """
        Upload firmware from a file.

        Args:
            path: Path to firmware binary file
            bank: Target bank (0=A, 1=B)
            version: Firmware version number
            chunk_size: Size of data chunks (default 1024)
            progress_callback: Optional callback(bytes_sent, total_bytes)

        Returns:
            CRC-32 checksum of uploaded firmware

        Raises:
            UploadError: If upload fails
            FileNotFoundError: If firmware file not found
        """
        firmware = Path(path).read_bytes()
        self.upload_firmware(firmware, bank, version, chunk_size, progress_callback)
        return crc32(firmware)

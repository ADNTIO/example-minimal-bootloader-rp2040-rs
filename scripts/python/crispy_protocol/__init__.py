# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
Crispy Bootloader Protocol - Python client library.

This package provides a Python interface to communicate with the
crispy-bootloader via USB CDC.

Example usage:
    from crispy_protocol import Transport, Command, Response

    with Transport("/dev/ttyACM0") as transport:
        # Get status
        status = transport.get_status()
        print(f"Active bank: {status.active_bank}")

        # Upload firmware
        transport.upload_firmware(
            firmware_path="firmware.bin",
            bank=0,
            version=1,
            progress_callback=lambda p: print(f"{p}%")
        )

        # Reboot
        transport.reboot()
"""

from .cobs import cobs_encode, cobs_decode
from .crc32 import crc32
from .protocol import (
    Command,
    CommandType,
    Response,
    ResponseType,
    AckStatus,
    BootState,
    StatusResponse,
    AckResponse,
    encode_get_status,
    encode_start_update,
    encode_data_block,
    encode_finish_update,
    encode_reboot,
    decode_response,
)
from .transport import (
    Transport,
    TransportError,
    TimeoutError,
    ProtocolError,
    UploadError,
)
from .varint import encode_varint, decode_varint

__version__ = "0.1.0"

__all__ = [
    # COBS
    "cobs_encode",
    "cobs_decode",
    # CRC
    "crc32",
    # Protocol types
    "Command",
    "CommandType",
    "Response",
    "ResponseType",
    "AckStatus",
    "BootState",
    "StatusResponse",
    "AckResponse",
    # Protocol encoding
    "encode_get_status",
    "encode_start_update",
    "encode_data_block",
    "encode_finish_update",
    "encode_reboot",
    "decode_response",
    # Transport
    "Transport",
    "TransportError",
    "TimeoutError",
    "ProtocolError",
    "UploadError",
    # Varint
    "encode_varint",
    "decode_varint",
]

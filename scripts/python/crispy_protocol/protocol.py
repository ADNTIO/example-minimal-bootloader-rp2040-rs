# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
Crispy bootloader protocol definitions and serialization.

This module defines the command/response protocol used to communicate
with the bootloader over USB CDC.
"""

from dataclasses import dataclass
from enum import IntEnum
from typing import Union

from .cobs import cobs_encode, cobs_decode
from .varint import encode_varint, decode_varint


class CommandType(IntEnum):
    """Command types (postcard enum variant indices)."""
    GET_STATUS = 0
    START_UPDATE = 1
    DATA_BLOCK = 2
    FINISH_UPDATE = 3
    REBOOT = 4
    SET_ACTIVE_BANK = 5
    WIPE_ALL = 6


class Command:
    """Command builder for bootloader protocol."""

    @staticmethod
    def get_status() -> bytes:
        """Create a GetStatus command."""
        return encode_get_status()

    @staticmethod
    def start_update(bank: int, size: int, crc32: int, version: int) -> bytes:
        """Create a StartUpdate command."""
        return encode_start_update(bank, size, crc32, version)

    @staticmethod
    def data_block(offset: int, data: bytes) -> bytes:
        """Create a DataBlock command."""
        return encode_data_block(offset, data)

    @staticmethod
    def finish_update() -> bytes:
        """Create a FinishUpdate command."""
        return encode_finish_update()

    @staticmethod
    def reboot() -> bytes:
        """Create a Reboot command."""
        return encode_reboot()

    @staticmethod
    def set_active_bank(bank: int) -> bytes:
        """Create a SetActiveBank command."""
        return encode_set_active_bank(bank)

    @staticmethod
    def wipe_all() -> bytes:
        """Create a WipeAll command."""
        return encode_wipe_all()


class AckStatus(IntEnum):
    """Acknowledgment status codes."""
    OK = 0
    CRC_ERROR = 1
    FLASH_ERROR = 2
    BAD_COMMAND = 3
    BAD_STATE = 4
    BANK_INVALID = 5

    def __str__(self) -> str:
        return self.name


class BootState(IntEnum):
    """Bootloader state."""
    IDLE = 0
    UPDATE_MODE = 1
    RECEIVING = 2

    def __str__(self) -> str:
        return self.name


class Response:
    """Response type constants."""
    TYPE_ACK = 0
    TYPE_STATUS = 1


@dataclass
class AckResponse:
    """Acknowledgment response from bootloader."""
    status: AckStatus
    type: int = Response.TYPE_ACK

    @property
    def is_ok(self) -> bool:
        return self.status == AckStatus.OK


@dataclass
class StatusResponse:
    """Status response from bootloader."""
    active_bank: int
    version_a: int
    version_b: int
    state: BootState
    type: int = Response.TYPE_STATUS

    @property
    def active_bank_name(self) -> str:
        return "A" if self.active_bank == 0 else "B"


# Type alias for any response
ResponseType = Union[AckResponse, StatusResponse]


def encode_get_status() -> bytes:
    """Encode a GetStatus command."""
    return _frame(bytes([CommandType.GET_STATUS]))


def encode_start_update(bank: int, size: int, crc32: int, version: int) -> bytes:
    """Encode a StartUpdate command."""
    payload = (
        bytes([CommandType.START_UPDATE, bank])
        + encode_varint(size)
        + encode_varint(crc32)
        + encode_varint(version)
    )
    return _frame(payload)


def encode_data_block(offset: int, data: bytes) -> bytes:
    """Encode a DataBlock command."""
    payload = (
        bytes([CommandType.DATA_BLOCK])
        + encode_varint(offset)
        + encode_varint(len(data))
        + data
    )
    return _frame(payload)


def encode_finish_update() -> bytes:
    """Encode a FinishUpdate command."""
    return _frame(bytes([CommandType.FINISH_UPDATE]))


def encode_reboot() -> bytes:
    """Encode a Reboot command."""
    return _frame(bytes([CommandType.REBOOT]))


def encode_set_active_bank(bank: int) -> bytes:
    """Encode a SetActiveBank command."""
    return _frame(bytes([CommandType.SET_ACTIVE_BANK, bank]))


def encode_wipe_all() -> bytes:
    """Encode a WipeAll command."""
    return _frame(bytes([CommandType.WIPE_ALL]))


def decode_response(data: bytes) -> ResponseType:
    """
    Decode a COBS-framed response.

    Args:
        data: Raw bytes received (with trailing 0x00 delimiter)

    Returns:
        Decoded response (AckResponse or StatusResponse)

    Raises:
        ValueError: If response is malformed
    """
    # Remove trailing delimiter if present
    if data and data[-1] == 0:
        data = data[:-1]

    decoded = cobs_decode(data)

    if len(decoded) < 1:
        raise ValueError("Empty response")

    resp_type = decoded[0]

    if resp_type == 0:  # Ack
        if len(decoded) < 2:
            raise ValueError("Truncated Ack response")
        return AckResponse(status=AckStatus(decoded[1]))

    elif resp_type == 1:  # Status
        if len(decoded) < 2:
            raise ValueError("Truncated Status response")

        active_bank = decoded[1]
        offset = 2
        version_a, offset = decode_varint(decoded, offset)
        version_b, offset = decode_varint(decoded, offset)

        if offset >= len(decoded):
            raise ValueError("Truncated Status response")
        state = BootState(decoded[offset])

        return StatusResponse(
            active_bank=active_bank,
            version_a=version_a,
            version_b=version_b,
            state=state,
        )

    else:
        raise ValueError(f"Unknown response type: {resp_type}")


def _frame(data: bytes) -> bytes:
    """Apply COBS encoding and add delimiter."""
    return cobs_encode(data) + b'\x00'

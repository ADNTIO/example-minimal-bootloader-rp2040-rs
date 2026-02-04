# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
CRC-32 (ISO HDLC / IEEE 802.3) implementation.

This is the same CRC-32 algorithm used by the bootloader for
firmware integrity verification.
"""

# Pre-computed CRC-32 lookup table
_CRC32_TABLE = []


def _init_table():
    """Initialize the CRC-32 lookup table."""
    global _CRC32_TABLE
    poly = 0xEDB88320
    for i in range(256):
        crc = i
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ poly
            else:
                crc >>= 1
        _CRC32_TABLE.append(crc)


_init_table()


def crc32(data: bytes) -> int:
    """
    Compute CRC-32 (ISO HDLC) checksum.

    Args:
        data: Bytes to compute checksum for

    Returns:
        32-bit CRC value
    """
    crc = 0xFFFFFFFF
    for byte in data:
        crc = _CRC32_TABLE[(crc ^ byte) & 0xFF] ^ (crc >> 8)
    return crc ^ 0xFFFFFFFF

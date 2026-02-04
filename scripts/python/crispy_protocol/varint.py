# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
Varint encoding/decoding (LEB128-style, postcard compatible).

Postcard uses unsigned LEB128 for variable-length integers.
"""

from typing import Tuple


def encode_varint(value: int) -> bytes:
    """
    Encode an unsigned integer as a varint.

    Args:
        value: Non-negative integer to encode

    Returns:
        Varint-encoded bytes
    """
    if value < 0:
        raise ValueError("Cannot encode negative value as varint")

    result = []
    while value >= 0x80:
        result.append((value & 0x7F) | 0x80)
        value >>= 7
    result.append(value)
    return bytes(result)


def decode_varint(data: bytes, offset: int = 0) -> Tuple[int, int]:
    """
    Decode a varint from bytes.

    Args:
        data: Bytes containing the varint
        offset: Starting offset in data

    Returns:
        Tuple of (decoded value, new offset after varint)

    Raises:
        ValueError: If varint is malformed or truncated
    """
    value = 0
    shift = 0

    while True:
        if offset >= len(data):
            raise ValueError("Varint decode: unexpected end of data")

        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift

        if not (byte & 0x80):
            break

        shift += 7
        if shift > 63:
            raise ValueError("Varint decode: value too large")

    return value, offset

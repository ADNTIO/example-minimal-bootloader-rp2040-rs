# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
COBS (Consistent Overhead Byte Stuffing) encoder/decoder.

COBS is a framing algorithm that eliminates 0x00 bytes from data,
allowing 0x00 to be used as a packet delimiter.
"""


def cobs_encode(data: bytes) -> bytes:
    """
    Encode data using COBS.

    Args:
        data: Raw bytes to encode

    Returns:
        COBS-encoded bytes (without delimiter)
    """
    output = bytearray()
    code_idx = 0
    code = 1
    output.append(0)  # Placeholder for first code byte

    for byte in data:
        if byte == 0:
            output[code_idx] = code
            code_idx = len(output)
            output.append(0)  # Placeholder
            code = 1
        else:
            output.append(byte)
            code += 1
            if code == 255:
                output[code_idx] = code
                code_idx = len(output)
                output.append(0)  # Placeholder
                code = 1

    output[code_idx] = code
    return bytes(output)


def cobs_decode(data: bytes) -> bytes:
    """
    Decode COBS-encoded data.

    Args:
        data: COBS-encoded bytes (with or without trailing delimiter)

    Returns:
        Decoded raw bytes

    Raises:
        ValueError: If data is malformed
    """
    output = bytearray()
    i = 0

    while i < len(data):
        code = data[i]
        if code == 0:
            break  # Delimiter
        i += 1

        for _ in range(1, code):
            if i >= len(data):
                raise ValueError("COBS decode: unexpected end of data")
            output.append(data[i])
            i += 1

        if code < 255 and i < len(data) and data[i] != 0:
            output.append(0)

    return bytes(output)

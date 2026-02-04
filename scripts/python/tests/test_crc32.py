# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Tests for CRC-32 calculation."""

import pytest
import zlib
from crispy_protocol.crc32 import crc32, _CRC32_TABLE, _init_table


class TestCrc32:
    """Tests for crc32 function."""

    def test_empty_data(self):
        """CRC of empty data."""
        assert crc32(b"") == 0x00000000

    def test_single_byte(self):
        """CRC of single byte."""
        # Verified against zlib.crc32
        assert crc32(b"\x00") == zlib.crc32(b"\x00") & 0xFFFFFFFF
        assert crc32(b"\xFF") == zlib.crc32(b"\xFF") & 0xFFFFFFFF
        assert crc32(b"A") == zlib.crc32(b"A") & 0xFFFFFFFF

    def test_known_values(self):
        """Known CRC-32 test vectors."""
        # "123456789" is a standard test vector
        assert crc32(b"123456789") == 0xCBF43926

        # Other known values
        assert crc32(b"test") == zlib.crc32(b"test") & 0xFFFFFFFF
        assert crc32(b"hello") == zlib.crc32(b"hello") & 0xFFFFFFFF

    def test_matches_zlib(self):
        """Our CRC-32 matches Python's zlib implementation."""
        test_cases = [
            b"",
            b"a",
            b"abc",
            b"Hello, World!",
            bytes(range(256)),
            b"\x00" * 100,
            b"\xFF" * 100,
        ]
        for data in test_cases:
            expected = zlib.crc32(data) & 0xFFFFFFFF
            assert crc32(data) == expected, f"Mismatch for {data!r}"

    def test_large_data(self):
        """CRC of large data matches zlib."""
        data = bytes(range(256)) * 1000  # 256KB
        expected = zlib.crc32(data) & 0xFFFFFFFF
        assert crc32(data) == expected

    def test_incremental_differs(self):
        """Different data produces different CRC."""
        assert crc32(b"test1") != crc32(b"test2")
        assert crc32(b"\x00") != crc32(b"\x01")

    def test_order_matters(self):
        """Byte order affects CRC."""
        assert crc32(b"ab") != crc32(b"ba")

    def test_returns_32bit_unsigned(self):
        """Result is always 32-bit unsigned."""
        test_cases = [b"", b"test", bytes(range(256))]
        for data in test_cases:
            result = crc32(data)
            assert 0 <= result <= 0xFFFFFFFF
            assert isinstance(result, int)


class TestCrc32Table:
    """Tests for CRC-32 lookup table."""

    def test_table_initialized(self):
        """Table is initialized with 256 entries."""
        assert len(_CRC32_TABLE) == 256

    def test_table_values_32bit(self):
        """All table values are 32-bit."""
        for value in _CRC32_TABLE:
            assert 0 <= value <= 0xFFFFFFFF

    def test_table_first_entry(self):
        """First entry is 0."""
        assert _CRC32_TABLE[0] == 0

    def test_table_deterministic(self):
        """Table values are deterministic."""
        # Verify known values at specific indices
        # These are derived from the polynomial 0xEDB88320
        assert _CRC32_TABLE[0] == 0x00000000
        assert _CRC32_TABLE[1] == 0x77073096
        assert _CRC32_TABLE[255] == 0x2D02EF8D

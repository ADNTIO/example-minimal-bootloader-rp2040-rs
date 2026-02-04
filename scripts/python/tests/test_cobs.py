# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Tests for COBS encoding/decoding."""

import pytest
from crispy_protocol.cobs import cobs_encode, cobs_decode


class TestCobsEncode:
    """Tests for cobs_encode function."""

    def test_empty_data(self):
        """Empty input produces single byte output."""
        assert cobs_encode(b"") == b"\x01"

    def test_single_nonzero_byte(self):
        """Single non-zero byte."""
        assert cobs_encode(b"\x11") == b"\x02\x11"

    def test_single_zero_byte(self):
        """Single zero byte."""
        assert cobs_encode(b"\x00") == b"\x01\x01"

    def test_multiple_zeros(self):
        """Multiple consecutive zeros."""
        assert cobs_encode(b"\x00\x00") == b"\x01\x01\x01"
        assert cobs_encode(b"\x00\x00\x00") == b"\x01\x01\x01\x01"

    def test_no_zeros(self):
        """Data without zeros."""
        assert cobs_encode(b"\x11\x22\x33") == b"\x04\x11\x22\x33"

    def test_zero_at_start(self):
        """Zero at the beginning."""
        assert cobs_encode(b"\x00\x11\x22") == b"\x01\x03\x11\x22"

    def test_zero_at_end(self):
        """Zero at the end."""
        assert cobs_encode(b"\x11\x22\x00") == b"\x03\x11\x22\x01"

    def test_zero_in_middle(self):
        """Zero in the middle."""
        assert cobs_encode(b"\x11\x00\x22") == b"\x02\x11\x02\x22"

    def test_standard_example(self):
        """Standard COBS example from Wikipedia."""
        # [0x11, 0x22, 0x00, 0x33] -> [0x03, 0x11, 0x22, 0x02, 0x33]
        assert cobs_encode(b"\x11\x22\x00\x33") == b"\x03\x11\x22\x02\x33"

    def test_254_bytes_no_zero(self):
        """254 bytes without zeros (max before code rollover)."""
        data = bytes(range(1, 255))  # 1-254
        encoded = cobs_encode(data)
        assert encoded[0] == 255  # Code for 254 bytes + implicit zero
        assert encoded[1:255] == data

    def test_255_bytes_triggers_rollover(self):
        """255 bytes triggers code byte rollover."""
        data = bytes([0x01] * 255)
        encoded = cobs_encode(data)
        # First code byte is 0xFF (254 data bytes follow)
        assert encoded[0] == 255
        # Then another code byte for remaining byte
        assert encoded[255] == 2  # 1 more data byte

    def test_long_data_with_zeros(self):
        """Long data with zeros interspersed."""
        data = b"\x01\x02\x03\x00\x04\x05\x00\x06"
        encoded = cobs_encode(data)
        decoded = cobs_decode(encoded)
        assert decoded == data


class TestCobsDecode:
    """Tests for cobs_decode function."""

    def test_empty_produces_empty(self):
        """Decoding minimal encoding produces empty."""
        assert cobs_decode(b"\x01") == b""

    def test_single_nonzero_byte(self):
        """Decode single non-zero byte."""
        assert cobs_decode(b"\x02\x11") == b"\x11"

    def test_single_zero_byte(self):
        """Decode single zero byte."""
        assert cobs_decode(b"\x01\x01") == b"\x00"

    def test_multiple_zeros(self):
        """Decode multiple zeros."""
        assert cobs_decode(b"\x01\x01\x01") == b"\x00\x00"

    def test_standard_example(self):
        """Standard COBS example."""
        assert cobs_decode(b"\x03\x11\x22\x02\x33") == b"\x11\x22\x00\x33"

    def test_with_trailing_delimiter(self):
        """Decode with trailing 0x00 delimiter."""
        assert cobs_decode(b"\x02\x11\x00") == b"\x11"

    def test_stops_at_delimiter(self):
        """Decoding stops at 0x00 delimiter."""
        # Extra data after delimiter is ignored
        assert cobs_decode(b"\x02\x11\x00\xFF\xFF") == b"\x11"

    def test_truncated_data_raises(self):
        """Truncated data raises ValueError."""
        with pytest.raises(ValueError, match="unexpected end of data"):
            cobs_decode(b"\x05\x11\x22")  # Code says 4 more bytes, only 2

    def test_empty_input(self):
        """Empty input returns empty output."""
        assert cobs_decode(b"") == b""

    def test_only_delimiter(self):
        """Only delimiter returns empty."""
        assert cobs_decode(b"\x00") == b""


class TestCobsRoundtrip:
    """Roundtrip tests for encode/decode."""

    @pytest.mark.parametrize("data", [
        b"",
        b"\x00",
        b"\x01",
        b"\xff",
        b"\x00\x00\x00",
        b"\x01\x02\x03",
        b"Hello, World!",
        b"\x00Hello\x00World\x00",
        bytes(range(256)),
        bytes([0x00] * 100),
        bytes([0xFF] * 100),
    ])
    def test_roundtrip(self, data):
        """Encode then decode returns original data."""
        encoded = cobs_encode(data)
        decoded = cobs_decode(encoded)
        assert decoded == data

    def test_roundtrip_large_data(self):
        """Roundtrip with large data."""
        data = bytes(range(256)) * 10  # 2560 bytes
        encoded = cobs_encode(data)
        decoded = cobs_decode(encoded)
        assert decoded == data

    def test_no_zeros_in_encoded(self):
        """Encoded data never contains 0x00."""
        test_cases = [
            b"\x00" * 100,
            bytes(range(256)),
            b"test\x00data\x00here",
        ]
        for data in test_cases:
            encoded = cobs_encode(data)
            assert 0 not in encoded

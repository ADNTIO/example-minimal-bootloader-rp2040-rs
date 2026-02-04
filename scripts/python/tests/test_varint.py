# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Tests for varint encoding/decoding."""

import pytest
from crispy_protocol.varint import encode_varint, decode_varint


class TestEncodeVarint:
    """Tests for encode_varint function."""

    def test_zero(self):
        """Encoding zero."""
        assert encode_varint(0) == b"\x00"

    def test_single_byte_values(self):
        """Values that fit in single byte (0-127)."""
        assert encode_varint(0) == b"\x00"
        assert encode_varint(1) == b"\x01"
        assert encode_varint(127) == b"\x7F"

    def test_two_byte_values(self):
        """Values requiring two bytes (128-16383)."""
        assert encode_varint(128) == b"\x80\x01"
        assert encode_varint(129) == b"\x81\x01"
        assert encode_varint(255) == b"\xFF\x01"
        assert encode_varint(256) == b"\x80\x02"
        assert encode_varint(16383) == b"\xFF\x7F"

    def test_three_byte_values(self):
        """Values requiring three bytes."""
        assert encode_varint(16384) == b"\x80\x80\x01"
        assert encode_varint(2097151) == b"\xFF\xFF\x7F"

    def test_four_byte_values(self):
        """Values requiring four bytes."""
        assert encode_varint(2097152) == b"\x80\x80\x80\x01"

    def test_large_values(self):
        """Large values (32-bit range)."""
        # 0xFFFFFFFF (max u32)
        result = encode_varint(0xFFFFFFFF)
        assert len(result) == 5
        # Decode should return same value
        decoded, _ = decode_varint(result)
        assert decoded == 0xFFFFFFFF

    def test_negative_raises(self):
        """Negative values raise ValueError."""
        with pytest.raises(ValueError, match="Cannot encode negative"):
            encode_varint(-1)
        with pytest.raises(ValueError, match="Cannot encode negative"):
            encode_varint(-100)

    def test_powers_of_two(self):
        """Powers of two encode correctly."""
        for i in range(32):
            value = 1 << i
            encoded = encode_varint(value)
            decoded, _ = decode_varint(encoded)
            assert decoded == value


class TestDecodeVarint:
    """Tests for decode_varint function."""

    def test_zero(self):
        """Decoding zero."""
        value, offset = decode_varint(b"\x00")
        assert value == 0
        assert offset == 1

    def test_single_byte_values(self):
        """Single byte varints."""
        value, offset = decode_varint(b"\x01")
        assert value == 1
        assert offset == 1

        value, offset = decode_varint(b"\x7F")
        assert value == 127
        assert offset == 1

    def test_two_byte_values(self):
        """Two byte varints."""
        value, offset = decode_varint(b"\x80\x01")
        assert value == 128
        assert offset == 2

        value, offset = decode_varint(b"\xFF\x01")
        assert value == 255
        assert offset == 2

    def test_with_offset(self):
        """Decoding with non-zero starting offset."""
        data = b"\xAA\xBB\x80\x01\xCC"
        value, new_offset = decode_varint(data, offset=2)
        assert value == 128
        assert new_offset == 4

    def test_with_trailing_data(self):
        """Trailing data is ignored."""
        data = b"\x01\xFF\xFF\xFF"
        value, offset = decode_varint(data)
        assert value == 1
        assert offset == 1

    def test_truncated_raises(self):
        """Truncated varint raises ValueError."""
        with pytest.raises(ValueError, match="unexpected end of data"):
            decode_varint(b"\x80")  # Continuation bit set but no next byte

        with pytest.raises(ValueError, match="unexpected end of data"):
            decode_varint(b"\x80\x80")  # Still needs more

    def test_empty_raises(self):
        """Empty data raises ValueError."""
        with pytest.raises(ValueError, match="unexpected end of data"):
            decode_varint(b"")

    def test_offset_past_end_raises(self):
        """Offset past end of data raises ValueError."""
        with pytest.raises(ValueError, match="unexpected end of data"):
            decode_varint(b"\x01", offset=5)

    def test_too_large_raises(self):
        """Varint that would overflow raises ValueError."""
        # 10 bytes with continuation bits set
        data = b"\x80\x80\x80\x80\x80\x80\x80\x80\x80\x80"
        with pytest.raises(ValueError, match="value too large"):
            decode_varint(data)


class TestVarintRoundtrip:
    """Roundtrip tests for encode/decode."""

    @pytest.mark.parametrize("value", [
        0, 1, 127, 128, 255, 256,
        16383, 16384,
        2097151, 2097152,
        0x7FFFFFFF,  # max i32
        0xFFFFFFFF,  # max u32
        0x123456789,  # > 32 bit
    ])
    def test_roundtrip(self, value):
        """Encode then decode returns original value."""
        encoded = encode_varint(value)
        decoded, offset = decode_varint(encoded)
        assert decoded == value
        assert offset == len(encoded)

    def test_roundtrip_all_single_byte(self):
        """All single-byte values roundtrip."""
        for i in range(128):
            encoded = encode_varint(i)
            assert len(encoded) == 1
            decoded, _ = decode_varint(encoded)
            assert decoded == i

    def test_consecutive_varints(self):
        """Multiple varints in sequence."""
        values = [0, 127, 128, 16384, 0xFFFFFFFF]
        data = b"".join(encode_varint(v) for v in values)

        offset = 0
        for expected in values:
            value, offset = decode_varint(data, offset)
            assert value == expected

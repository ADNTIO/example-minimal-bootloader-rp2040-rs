# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Tests for protocol encoding/decoding."""

import pytest
from crispy_protocol.protocol import (
    CommandType,
    AckStatus,
    BootState,
    AckResponse,
    StatusResponse,
    encode_get_status,
    encode_start_update,
    encode_data_block,
    encode_finish_update,
    encode_reboot,
    encode_set_active_bank,
    encode_wipe_all,
    decode_response,
    _frame,
)
from crispy_protocol.cobs import cobs_decode


class TestCommandEnum:
    """Tests for CommandType enum."""

    def test_values(self):
        """CommandType enum has correct values."""
        assert CommandType.GET_STATUS == 0
        assert CommandType.START_UPDATE == 1
        assert CommandType.DATA_BLOCK == 2
        assert CommandType.FINISH_UPDATE == 3
        assert CommandType.REBOOT == 4
        assert CommandType.SET_ACTIVE_BANK == 5
        assert CommandType.WIPE_ALL == 6

    def test_all_members(self):
        """All expected commands exist."""
        assert len(CommandType) == 7


class TestAckStatusEnum:
    """Tests for AckStatus enum."""

    def test_values(self):
        """AckStatus enum has correct values."""
        assert AckStatus.OK == 0
        assert AckStatus.CRC_ERROR == 1
        assert AckStatus.FLASH_ERROR == 2
        assert AckStatus.BAD_COMMAND == 3
        assert AckStatus.BAD_STATE == 4
        assert AckStatus.BANK_INVALID == 5

    def test_str(self):
        """AckStatus __str__ returns name."""
        assert str(AckStatus.OK) == "OK"
        assert str(AckStatus.CRC_ERROR) == "CRC_ERROR"


class TestBootStateEnum:
    """Tests for BootState enum."""

    def test_values(self):
        """BootState enum has correct values."""
        assert BootState.IDLE == 0
        assert BootState.UPDATE_MODE == 1
        assert BootState.RECEIVING == 2

    def test_str(self):
        """BootState __str__ returns name."""
        assert str(BootState.IDLE) == "IDLE"
        assert str(BootState.UPDATE_MODE) == "UPDATE_MODE"


class TestAckResponse:
    """Tests for AckResponse dataclass."""

    def test_is_ok_true(self):
        """is_ok returns True for OK status."""
        resp = AckResponse(status=AckStatus.OK)
        assert resp.is_ok is True

    def test_is_ok_false(self):
        """is_ok returns False for non-OK status."""
        for status in AckStatus:
            if status != AckStatus.OK:
                resp = AckResponse(status=status)
                assert resp.is_ok is False


class TestStatusResponse:
    """Tests for StatusResponse dataclass."""

    def test_active_bank_name_a(self):
        """active_bank_name returns 'A' for bank 0."""
        resp = StatusResponse(active_bank=0, version_a=1, version_b=2, state=BootState.IDLE)
        assert resp.active_bank_name == "A"

    def test_active_bank_name_b(self):
        """active_bank_name returns 'B' for bank 1."""
        resp = StatusResponse(active_bank=1, version_a=1, version_b=2, state=BootState.IDLE)
        assert resp.active_bank_name == "B"

    def test_fields(self):
        """All fields are accessible."""
        resp = StatusResponse(active_bank=1, version_a=5, version_b=3, state=BootState.UPDATE_MODE)
        assert resp.active_bank == 1
        assert resp.version_a == 5
        assert resp.version_b == 3
        assert resp.state == BootState.UPDATE_MODE


class TestFrame:
    """Tests for _frame helper function."""

    def test_adds_cobs_and_delimiter(self):
        """_frame applies COBS and adds 0x00 delimiter."""
        framed = _frame(b"\x01\x02\x03")
        assert framed[-1] == 0  # Ends with delimiter
        # Decode should give back original
        decoded = cobs_decode(framed[:-1])
        assert decoded == b"\x01\x02\x03"


class TestEncodeGetStatus:
    """Tests for encode_get_status."""

    def test_encodes_correctly(self):
        """GetStatus command encodes correctly."""
        encoded = encode_get_status()
        assert encoded[-1] == 0  # COBS delimiter

        # Decode and verify
        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.GET_STATUS])


class TestEncodeStartUpdate:
    """Tests for encode_start_update."""

    def test_encodes_small_values(self):
        """StartUpdate with small values."""
        encoded = encode_start_update(bank=0, size=100, crc32=0x12345678, version=1)
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded[0] == CommandType.START_UPDATE
        assert decoded[1] == 0  # bank

    def test_encodes_bank_b(self):
        """StartUpdate for bank B."""
        encoded = encode_start_update(bank=1, size=1024, crc32=0, version=5)
        decoded = cobs_decode(encoded[:-1])
        assert decoded[1] == 1  # bank B

    def test_encodes_large_size(self):
        """StartUpdate with large size value."""
        encoded = encode_start_update(bank=0, size=786432, crc32=0xDEADBEEF, version=100)
        decoded = cobs_decode(encoded[:-1])
        assert decoded[0] == CommandType.START_UPDATE
        # Varints should decode correctly (tested via roundtrip)


class TestEncodeDataBlock:
    """Tests for encode_data_block."""

    def test_encodes_small_block(self):
        """DataBlock with small data."""
        data = b"\x11\x22\x33\x44"
        encoded = encode_data_block(offset=0, data=data)
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded[0] == CommandType.DATA_BLOCK

    def test_encodes_with_offset(self):
        """DataBlock with non-zero offset."""
        data = b"\xAA" * 100
        encoded = encode_data_block(offset=1024, data=data)
        decoded = cobs_decode(encoded[:-1])
        assert decoded[0] == CommandType.DATA_BLOCK

    def test_encodes_max_chunk(self):
        """DataBlock with max chunk size (1024 bytes)."""
        data = b"\xFF" * 1024
        encoded = encode_data_block(offset=0, data=data)
        decoded = cobs_decode(encoded[:-1])
        assert decoded[0] == CommandType.DATA_BLOCK
        # Data should be at the end
        assert data in decoded

    def test_encodes_data_with_zeros(self):
        """DataBlock with zeros in data."""
        data = b"\x00\x11\x00\x22\x00"
        encoded = encode_data_block(offset=0, data=data)
        # COBS ensures no zeros in encoded (except delimiter)
        assert encoded.count(0) == 1  # Only the delimiter


class TestEncodeFinishUpdate:
    """Tests for encode_finish_update."""

    def test_encodes_correctly(self):
        """FinishUpdate command encodes correctly."""
        encoded = encode_finish_update()
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.FINISH_UPDATE])


class TestEncodeReboot:
    """Tests for encode_reboot."""

    def test_encodes_correctly(self):
        """Reboot command encodes correctly."""
        encoded = encode_reboot()
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.REBOOT])


class TestEncodeSetActiveBank:
    """Tests for encode_set_active_bank."""

    def test_encodes_bank_a(self):
        """SetActiveBank for bank A encodes correctly."""
        encoded = encode_set_active_bank(bank=0)
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.SET_ACTIVE_BANK, 0])

    def test_encodes_bank_b(self):
        """SetActiveBank for bank B encodes correctly."""
        encoded = encode_set_active_bank(bank=1)
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.SET_ACTIVE_BANK, 1])


class TestEncodeWipeAll:
    """Tests for encode_wipe_all."""

    def test_encodes_correctly(self):
        """WipeAll command encodes correctly."""
        encoded = encode_wipe_all()
        assert encoded[-1] == 0

        decoded = cobs_decode(encoded[:-1])
        assert decoded == bytes([CommandType.WIPE_ALL])


class TestDecodeResponse:
    """Tests for decode_response."""

    def test_decode_ack_ok(self):
        """Decode Ack response with OK status."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([0, AckStatus.OK])  # Type 0 = Ack
        framed = cobs_encode(raw) + b"\x00"

        resp = decode_response(framed)
        assert isinstance(resp, AckResponse)
        assert resp.status == AckStatus.OK
        assert resp.is_ok is True

    def test_decode_ack_error(self):
        """Decode Ack response with error status."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([0, AckStatus.CRC_ERROR])
        framed = cobs_encode(raw) + b"\x00"

        resp = decode_response(framed)
        assert isinstance(resp, AckResponse)
        assert resp.status == AckStatus.CRC_ERROR
        assert resp.is_ok is False

    def test_decode_status_response(self):
        """Decode Status response."""
        from crispy_protocol.cobs import cobs_encode
        from crispy_protocol.varint import encode_varint

        # Build Status response: type=1, active_bank, version_a, version_b, state
        raw = (
            bytes([1, 0])  # Type 1 = Status, bank 0
            + encode_varint(5)  # version_a = 5
            + encode_varint(3)  # version_b = 3
            + bytes([BootState.UPDATE_MODE])
        )
        framed = cobs_encode(raw) + b"\x00"

        resp = decode_response(framed)
        assert isinstance(resp, StatusResponse)
        assert resp.active_bank == 0
        assert resp.version_a == 5
        assert resp.version_b == 3
        assert resp.state == BootState.UPDATE_MODE

    def test_decode_status_bank_b(self):
        """Decode Status response for bank B."""
        from crispy_protocol.cobs import cobs_encode
        from crispy_protocol.varint import encode_varint

        raw = (
            bytes([1, 1])  # Type 1 = Status, bank 1
            + encode_varint(10)
            + encode_varint(20)
            + bytes([BootState.IDLE])
        )
        framed = cobs_encode(raw) + b"\x00"

        resp = decode_response(framed)
        assert resp.active_bank == 1
        assert resp.active_bank_name == "B"
        assert resp.version_a == 10
        assert resp.version_b == 20

    def test_decode_without_delimiter(self):
        """Decode response without trailing delimiter."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([0, AckStatus.OK])
        framed = cobs_encode(raw)  # No trailing 0x00

        resp = decode_response(framed)
        assert isinstance(resp, AckResponse)
        assert resp.is_ok is True

    def test_decode_empty_raises(self):
        """Empty response raises ValueError."""
        from crispy_protocol.cobs import cobs_encode
        framed = cobs_encode(b"") + b"\x00"

        with pytest.raises(ValueError, match="Empty response"):
            decode_response(framed)

    def test_decode_truncated_ack_raises(self):
        """Truncated Ack response raises ValueError."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([0])  # Type only, no status
        framed = cobs_encode(raw) + b"\x00"

        with pytest.raises(ValueError, match="Truncated Ack"):
            decode_response(framed)

    def test_decode_truncated_status_raises(self):
        """Truncated Status response raises ValueError."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([1])  # Type only
        framed = cobs_encode(raw) + b"\x00"

        with pytest.raises(ValueError, match="Truncated Status"):
            decode_response(framed)

    def test_decode_unknown_type_raises(self):
        """Unknown response type raises ValueError."""
        from crispy_protocol.cobs import cobs_encode
        raw = bytes([99, 0, 0])  # Unknown type 99
        framed = cobs_encode(raw) + b"\x00"

        with pytest.raises(ValueError, match="Unknown response type"):
            decode_response(framed)

    def test_decode_large_versions(self):
        """Decode Status with large version numbers."""
        from crispy_protocol.cobs import cobs_encode
        from crispy_protocol.varint import encode_varint

        raw = (
            bytes([1, 0])
            + encode_varint(0xFFFFFFFF)  # Max u32
            + encode_varint(0x12345678)
            + bytes([BootState.RECEIVING])
        )
        framed = cobs_encode(raw) + b"\x00"

        resp = decode_response(framed)
        assert resp.version_a == 0xFFFFFFFF
        assert resp.version_b == 0x12345678
        assert resp.state == BootState.RECEIVING

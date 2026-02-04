# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Tests for Transport class."""

import pytest
from unittest.mock import Mock, patch, MagicMock
from io import BytesIO

from crispy_protocol.transport import (
    Transport,
    TransportError,
    TimeoutError,
    ProtocolError,
    UploadError,
)
from crispy_protocol.protocol import (
    AckStatus,
    BootState,
    AckResponse,
    StatusResponse,
)
from crispy_protocol.cobs import cobs_encode
from crispy_protocol.varint import encode_varint
from crispy_protocol.crc32 import crc32


def make_ack_response(status: AckStatus) -> bytes:
    """Create a framed Ack response."""
    raw = bytes([0, status])  # Type 0 = Ack
    return cobs_encode(raw) + b"\x00"


def make_status_response(
    active_bank: int,
    version_a: int,
    version_b: int,
    state: BootState
) -> bytes:
    """Create a framed Status response."""
    raw = (
        bytes([1, active_bank])
        + encode_varint(version_a)
        + encode_varint(version_b)
        + bytes([state])
    )
    return cobs_encode(raw) + b"\x00"


class MockSerial:
    """Mock serial port for testing."""

    def __init__(self, responses: list[bytes] = None):
        self.responses = responses or []
        self.response_idx = 0
        self.written = BytesIO()
        self.is_open = True
        self.port = "/dev/ttyTEST"

    def read(self, size: int) -> bytes:
        """Read bytes, returning from response queue."""
        if self.response_idx >= len(self.responses):
            return b""  # Timeout

        resp = self.responses[self.response_idx]
        if not hasattr(self, '_resp_offset'):
            self._resp_offset = 0

        # Return one byte at a time
        if self._resp_offset < len(resp):
            byte = bytes([resp[self._resp_offset]])
            self._resp_offset += 1
            if self._resp_offset >= len(resp):
                self.response_idx += 1
                self._resp_offset = 0
            return byte
        return b""

    def write(self, data: bytes) -> int:
        self.written.write(data)
        return len(data)

    def flush(self):
        pass

    def close(self):
        self.is_open = False


class TestTransportInit:
    """Tests for Transport initialization."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_init_opens_serial(self, mock_sleep, mock_serial_class):
        """Transport opens serial port on init."""
        mock_serial = Mock()
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        mock_serial_class.assert_called_once_with(
            "/dev/ttyACM0", 115200, timeout=5.0
        )
        mock_sleep.assert_called_once_with(0.1)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_init_custom_params(self, mock_sleep, mock_serial_class):
        """Transport accepts custom baudrate and timeout."""
        mock_serial = Mock()
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyUSB0", baudrate=9600, timeout=10.0)

        mock_serial_class.assert_called_once_with(
            "/dev/ttyUSB0", 9600, timeout=10.0
        )


class TestTransportContextManager:
    """Tests for context manager functionality."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_context_manager_closes(self, mock_sleep, mock_serial_class):
        """Context manager closes serial on exit."""
        mock_serial = Mock()
        mock_serial.is_open = True
        mock_serial_class.return_value = mock_serial

        with Transport("/dev/ttyACM0") as t:
            pass

        mock_serial.close.assert_called_once()

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_context_manager_closes_on_exception(self, mock_sleep, mock_serial_class):
        """Context manager closes serial even on exception."""
        mock_serial = Mock()
        mock_serial.is_open = True
        mock_serial_class.return_value = mock_serial

        with pytest.raises(RuntimeError):
            with Transport("/dev/ttyACM0") as t:
                raise RuntimeError("test")

        mock_serial.close.assert_called_once()


class TestTransportClose:
    """Tests for close method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_close_closes_serial(self, mock_sleep, mock_serial_class):
        """close() closes the serial port."""
        mock_serial = Mock()
        mock_serial.is_open = True
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        t.close()

        mock_serial.close.assert_called_once()

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_close_handles_already_closed(self, mock_sleep, mock_serial_class):
        """close() handles already closed port."""
        mock_serial = Mock()
        mock_serial.is_open = False
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        t.close()  # Should not raise

        mock_serial.close.assert_not_called()


class TestTransportPort:
    """Tests for port property."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_port_property(self, mock_sleep, mock_serial_class):
        """port property returns serial port name."""
        mock_serial = Mock()
        mock_serial.port = "/dev/ttyACM0"
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        assert t.port == "/dev/ttyACM0"


class TestTransportGetStatus:
    """Tests for get_status method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_get_status_success(self, mock_sleep, mock_serial_class):
        """get_status returns StatusResponse."""
        response = make_status_response(0, 5, 3, BootState.IDLE)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        status = t.get_status()

        assert isinstance(status, StatusResponse)
        assert status.active_bank == 0
        assert status.version_a == 5
        assert status.version_b == 3
        assert status.state == BootState.IDLE

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_get_status_wrong_response_type(self, mock_sleep, mock_serial_class):
        """get_status raises ProtocolError for wrong response type."""
        response = make_ack_response(AckStatus.OK)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(ProtocolError, match="Expected StatusResponse"):
            t.get_status()


class TestTransportStartUpdate:
    """Tests for start_update method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_start_update_success(self, mock_sleep, mock_serial_class):
        """start_update returns AckResponse."""
        response = make_ack_response(AckStatus.OK)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        resp = t.start_update(bank=0, size=1024, crc=0x12345678, version=1)

        assert isinstance(resp, AckResponse)
        assert resp.is_ok is True

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_start_update_wrong_response_type(self, mock_sleep, mock_serial_class):
        """start_update raises ProtocolError for wrong response type."""
        response = make_status_response(0, 1, 1, BootState.IDLE)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(ProtocolError, match="Expected AckResponse"):
            t.start_update(bank=0, size=1024, crc=0, version=1)


class TestTransportSendDataBlock:
    """Tests for send_data_block method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_send_data_block_success(self, mock_sleep, mock_serial_class):
        """send_data_block returns AckResponse."""
        response = make_ack_response(AckStatus.OK)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        resp = t.send_data_block(offset=0, data=b"\x11\x22\x33")

        assert resp.is_ok is True

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_send_data_block_wrong_response_type(self, mock_sleep, mock_serial_class):
        """send_data_block raises ProtocolError for wrong response type."""
        response = make_status_response(0, 1, 1, BootState.IDLE)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(ProtocolError, match="Expected AckResponse"):
            t.send_data_block(offset=0, data=b"\xFF")


class TestTransportFinishUpdate:
    """Tests for finish_update method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_finish_update_success(self, mock_sleep, mock_serial_class):
        """finish_update returns AckResponse."""
        response = make_ack_response(AckStatus.OK)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        resp = t.finish_update()

        assert resp.is_ok is True

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_finish_update_crc_error(self, mock_sleep, mock_serial_class):
        """finish_update handles CRC error."""
        response = make_ack_response(AckStatus.CRC_ERROR)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        resp = t.finish_update()

        assert resp.is_ok is False
        assert resp.status == AckStatus.CRC_ERROR


class TestTransportReboot:
    """Tests for reboot method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_reboot_success(self, mock_sleep, mock_serial_class):
        """reboot returns AckResponse."""
        response = make_ack_response(AckStatus.OK)
        mock_serial = MockSerial([response])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")
        resp = t.reboot()

        assert resp.is_ok is True


class TestTransportReceive:
    """Tests for _receive method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_receive_timeout(self, mock_sleep, mock_serial_class):
        """_receive raises TimeoutError on timeout."""
        mock_serial = MockSerial([])  # No responses
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(TimeoutError, match="Timeout waiting for response"):
            t._receive()


class TestTransportUploadFirmware:
    """Tests for upload_firmware method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_success(self, mock_sleep, mock_serial_class):
        """upload_firmware completes successfully."""
        # Responses: start_update OK, data_block OK (x2), finish_update OK
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.OK),  # data_block 1
            make_ack_response(AckStatus.OK),  # data_block 2
            make_ack_response(AckStatus.OK),  # finish_update
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        firmware = b"\xFF" * 1500  # 2 chunks
        t.upload_firmware(firmware, bank=0, version=1, chunk_size=1024)
        # Should complete without exception

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_start_fails(self, mock_sleep, mock_serial_class):
        """upload_firmware raises UploadError if start fails."""
        responses = [make_ack_response(AckStatus.BANK_INVALID)]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(UploadError, match="StartUpdate failed"):
            t.upload_firmware(b"\xFF" * 100, bank=0, version=1)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_data_block_fails(self, mock_sleep, mock_serial_class):
        """upload_firmware raises UploadError if data block fails."""
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.FLASH_ERROR),  # data_block fails
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(UploadError, match="DataBlock failed"):
            t.upload_firmware(b"\xFF" * 100, bank=0, version=1)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_finish_crc_error(self, mock_sleep, mock_serial_class):
        """upload_firmware raises UploadError on CRC error."""
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.OK),  # data_block
            make_ack_response(AckStatus.CRC_ERROR),  # finish_update
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(UploadError, match="CRC verification failed"):
            t.upload_firmware(b"\xFF" * 100, bank=0, version=1)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_finish_other_error(self, mock_sleep, mock_serial_class):
        """upload_firmware raises UploadError on finish error."""
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.OK),  # data_block
            make_ack_response(AckStatus.BAD_STATE),  # finish_update
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(UploadError, match="FinishUpdate failed"):
            t.upload_firmware(b"\xFF" * 100, bank=0, version=1)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_with_progress(self, mock_sleep, mock_serial_class):
        """upload_firmware calls progress callback."""
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.OK),  # data_block
            make_ack_response(AckStatus.OK),  # finish_update
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        progress_calls = []
        t.upload_firmware(
            b"\xFF" * 100,
            bank=0,
            version=1,
            progress_callback=lambda sent, total: progress_calls.append((sent, total))
        )

        assert len(progress_calls) == 1
        assert progress_calls[0] == (100, 100)


class TestTransportUploadFirmwareFile:
    """Tests for upload_firmware_file method."""

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_file_success(self, mock_sleep, mock_serial_class, tmp_path):
        """upload_firmware_file reads file and uploads."""
        responses = [
            make_ack_response(AckStatus.OK),  # start_update
            make_ack_response(AckStatus.OK),  # data_block
            make_ack_response(AckStatus.OK),  # finish_update
        ]
        mock_serial = MockSerial(responses)
        mock_serial_class.return_value = mock_serial

        # Create temp firmware file
        fw_path = tmp_path / "firmware.bin"
        fw_data = b"\xDE\xAD\xBE\xEF" * 25
        fw_path.write_bytes(fw_data)

        t = Transport("/dev/ttyACM0")
        result_crc = t.upload_firmware_file(fw_path, bank=0, version=1)

        assert result_crc == crc32(fw_data)

    @patch('crispy_protocol.transport.serial.Serial')
    @patch('crispy_protocol.transport.time.sleep')
    def test_upload_firmware_file_not_found(self, mock_sleep, mock_serial_class, tmp_path):
        """upload_firmware_file raises for missing file."""
        mock_serial = MockSerial([])
        mock_serial_class.return_value = mock_serial

        t = Transport("/dev/ttyACM0")

        with pytest.raises(FileNotFoundError):
            t.upload_firmware_file(tmp_path / "nonexistent.bin", bank=0, version=1)


class TestExceptions:
    """Tests for exception classes."""

    def test_transport_error_is_exception(self):
        """TransportError inherits from Exception."""
        assert issubclass(TransportError, Exception)

    def test_timeout_error_is_transport_error(self):
        """TimeoutError inherits from TransportError."""
        assert issubclass(TimeoutError, TransportError)

    def test_protocol_error_is_transport_error(self):
        """ProtocolError inherits from TransportError."""
        assert issubclass(ProtocolError, TransportError)

    def test_upload_error_is_transport_error(self):
        """UploadError inherits from TransportError."""
        assert issubclass(UploadError, TransportError)

    def test_exceptions_can_have_message(self):
        """All exceptions can carry a message."""
        assert str(TransportError("test")) == "test"
        assert str(TimeoutError("timeout msg")) == "timeout msg"
        assert str(ProtocolError("proto msg")) == "proto msg"
        assert str(UploadError("upload msg")) == "upload msg"

# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
BDD-style integration tests for crispy-bootloader.

These tests require a physical RP2040 device connected via USB.
Run with: pytest tests/test_integration.py -v --device /dev/ttyACM0

Features tested:
- Bootloader status query
- Firmware upload to bank A/B
- CRC verification
- Reboot functionality
- Bank switching
"""

import os
import subprocess
import time
from pathlib import Path

import pytest

# Skip all tests if no device specified
pytestmark = pytest.mark.integration


class TestBuildArtifacts:
    """Feature: Build bootloader and firmware artifacts."""

    def test_build_bootloader(self, project_root, skip_build):
        """Scenario: Build the bootloader."""
        if skip_build:
            pytest.skip("Build skipped")

        result = subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "-p",
                "crispy-bootloader",
                "--target",
                "thumbv6m-none-eabi",
            ],
            cwd=project_root,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, f"Build failed: {result.stderr}"

    def test_build_firmware(self, project_root, skip_build):
        """Scenario: Build the sample firmware."""
        if skip_build:
            pytest.skip("Build skipped")

        result = subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "-p",
                "crispy-fw-sample-rs",
                "--target",
                "thumbv6m-none-eabi",
            ],
            cwd=project_root,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, f"Build failed: {result.stderr}"

    def test_create_firmware_binary(self, project_root, skip_build):
        """Scenario: Create firmware binary from ELF."""
        if skip_build:
            pytest.skip("Build skipped")

        elf_path = (
            project_root
            / "target"
            / "thumbv6m-none-eabi"
            / "release"
            / "crispy-fw-sample-rs"
        )
        bin_path = project_root / "target" / "firmware.bin"

        result = subprocess.run(
            ["arm-none-eabi-objcopy", "-O", "binary", str(elf_path), str(bin_path)],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, f"objcopy failed: {result.stderr}"
        assert bin_path.exists(), "Firmware binary not created"

    def test_firmware_size(self, project_root):
        """Scenario: Firmware size is within bank limits."""
        bin_path = project_root / "target" / "firmware.bin"
        if not bin_path.exists():
            pytest.skip("Firmware binary not found")

        max_size = 768 * 1024  # 768KB bank size
        actual_size = bin_path.stat().st_size

        assert actual_size < max_size, f"Firmware too large: {actual_size} > {max_size}"
        print(f"Firmware size: {actual_size} bytes ({actual_size / 1024:.1f} KB)")


class TestBootloaderStatus:
    """Feature: Query bootloader status."""

    def test_get_status(self, transport):
        """Scenario: Get bootloader status when in update mode."""
        from crispy_protocol.protocol import Command, Response

        # Given the device is in update mode
        # When I send a GetStatus command
        transport.send(Command.get_status())

        # Then I receive a Status response
        response = transport.receive()
        assert response is not None, "No response received"
        assert response.type == Response.TYPE_STATUS, f"Expected Status, got {response}"

        print(f"Active bank: {response.active_bank}")
        print(f"Version A: {response.version_a}")
        print(f"Version B: {response.version_b}")
        print(f"State: {response.state}")

    def test_status_shows_update_mode(self, transport):
        """Scenario: Status indicates update mode."""
        from crispy_protocol.protocol import BootState, Command, Response

        transport.send(Command.get_status())
        response = transport.receive()

        assert response.state in (
            BootState.UPDATE_MODE,
            BootState.RECEIVING,
        ), f"Expected UpdateMode or Receiving, got {response.state}"


class TestFirmwareUpload:
    """Feature: Upload firmware to device."""

    @pytest.fixture
    def firmware_path(self, project_root):
        path = project_root / "target" / "firmware.bin"
        if not path.exists():
            pytest.skip("Firmware binary not found. Run build tests first.")
        return path

    @pytest.fixture
    def firmware_data(self, firmware_path):
        return firmware_path.read_bytes()

    def test_start_update_bank_a(self, transport, firmware_data):
        """Scenario: Start firmware update to bank A."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command, Response

        # Given I have firmware data
        size = len(firmware_data)
        checksum = crc32(firmware_data)

        # When I send StartUpdate for bank A
        transport.send(Command.start_update(bank=0, size=size, crc32=checksum, version=1))

        # Then I receive an OK acknowledgment
        response = transport.receive()
        assert response is not None, "No response received"
        assert response.type == Response.TYPE_ACK, f"Expected Ack, got {response}"
        assert response.status == AckStatus.OK, f"Expected OK, got {response.status}"

    def test_upload_data_blocks(self, transport, firmware_data):
        """Scenario: Upload firmware data in blocks."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command, Response

        size = len(firmware_data)
        checksum = crc32(firmware_data)
        chunk_size = 1024

        # Start update
        transport.send(Command.start_update(bank=0, size=size, crc32=checksum, version=2))
        response = transport.receive()
        assert response.status == AckStatus.OK, f"StartUpdate failed: {response.status}"

        # Upload data blocks
        offset = 0
        while offset < size:
            chunk = firmware_data[offset : offset + chunk_size]
            transport.send(Command.data_block(offset=offset, data=chunk))

            response = transport.receive()
            assert response is not None, f"No response at offset {offset}"
            assert response.type == Response.TYPE_ACK, f"Expected Ack at offset {offset}"
            assert (
                response.status == AckStatus.OK
            ), f"DataBlock failed at {offset}: {response.status}"

            offset += len(chunk)

        print(f"Uploaded {offset} bytes in {offset // chunk_size + 1} blocks")

    def test_finish_update(self, transport, firmware_data):
        """Scenario: Finish firmware update and verify CRC."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command, Response

        size = len(firmware_data)
        checksum = crc32(firmware_data)
        chunk_size = 1024

        # Start and upload
        transport.send(Command.start_update(bank=0, size=size, crc32=checksum, version=3))
        assert transport.receive().status == AckStatus.OK

        offset = 0
        while offset < size:
            chunk = firmware_data[offset : offset + chunk_size]
            transport.send(Command.data_block(offset=offset, data=chunk))
            assert transport.receive().status == AckStatus.OK
            offset += len(chunk)

        # Finish update
        transport.send(Command.finish_update())
        response = transport.receive()

        assert response is not None, "No response to FinishUpdate"
        assert response.type == Response.TYPE_ACK, f"Expected Ack, got {response}"
        assert response.status == AckStatus.OK, f"FinishUpdate failed: {response.status}"

        print("Firmware update completed successfully!")

    def test_status_after_upload(self, transport, firmware_data):
        """Scenario: Status reflects uploaded firmware version."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command, Response

        size = len(firmware_data)
        checksum = crc32(firmware_data)
        version = 42

        # Upload to bank A with specific version
        transport.send(Command.start_update(bank=0, size=size, crc32=checksum, version=version))
        assert transport.receive().status == AckStatus.OK

        offset = 0
        while offset < size:
            chunk = firmware_data[offset : offset + 1024]
            transport.send(Command.data_block(offset=offset, data=chunk))
            assert transport.receive().status == AckStatus.OK
            offset += len(chunk)

        transport.send(Command.finish_update())
        assert transport.receive().status == AckStatus.OK

        # Check status
        transport.send(Command.get_status())
        response = transport.receive()

        assert response.active_bank == 0, f"Expected bank 0, got {response.active_bank}"
        assert response.version_a == version, f"Expected version {version}, got {response.version_a}"


class TestBankSwitching:
    """Feature: Switch between firmware banks."""

    @pytest.fixture
    def firmware_data(self, project_root):
        path = project_root / "target" / "firmware.bin"
        if not path.exists():
            pytest.skip("Firmware binary not found")
        return path.read_bytes()

    def test_upload_to_bank_b(self, transport, firmware_data):
        """Scenario: Upload firmware to bank B."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command

        size = len(firmware_data)
        checksum = crc32(firmware_data)
        version = 100

        # Upload to bank B
        transport.send(Command.start_update(bank=1, size=size, crc32=checksum, version=version))
        assert transport.receive().status == AckStatus.OK

        offset = 0
        while offset < size:
            chunk = firmware_data[offset : offset + 1024]
            transport.send(Command.data_block(offset=offset, data=chunk))
            assert transport.receive().status == AckStatus.OK
            offset += len(chunk)

        transport.send(Command.finish_update())
        assert transport.receive().status == AckStatus.OK

        # Verify bank B is now active
        transport.send(Command.get_status())
        response = transport.receive()

        assert response.active_bank == 1, f"Expected bank 1, got {response.active_bank}"
        assert response.version_b == version, f"Expected version {version}, got {response.version_b}"


class TestErrorHandling:
    """Feature: Handle error conditions gracefully."""

    def test_invalid_bank(self, transport):
        """Scenario: Reject invalid bank number."""
        from crispy_protocol.protocol import AckStatus, Command

        transport.send(Command.start_update(bank=2, size=1024, crc32=0, version=1))
        response = transport.receive()

        assert response.status == AckStatus.BANK_INVALID

    def test_zero_size(self, transport):
        """Scenario: Reject zero-size firmware."""
        from crispy_protocol.protocol import AckStatus, Command

        transport.send(Command.start_update(bank=0, size=0, crc32=0, version=1))
        response = transport.receive()

        assert response.status == AckStatus.BANK_INVALID

    def test_data_block_without_start(self, transport):
        """Scenario: Reject data block without StartUpdate."""
        from crispy_protocol.protocol import AckStatus, Command

        # Ensure we're in Idle state by getting status first
        transport.send(Command.get_status())
        transport.receive()

        # Try to send data without starting
        transport.send(Command.data_block(offset=0, data=b"\x00" * 256))
        response = transport.receive()

        assert response.status == AckStatus.BAD_STATE

    def test_wrong_offset(self, transport):
        """Scenario: Reject data block with wrong offset."""
        from crispy_protocol.crc32 import crc32
        from crispy_protocol.protocol import AckStatus, Command

        data = b"\x00" * 2048
        transport.send(Command.start_update(bank=0, size=len(data), crc32=crc32(data), version=1))
        assert transport.receive().status == AckStatus.OK

        # Send first block
        transport.send(Command.data_block(offset=0, data=data[:1024]))
        assert transport.receive().status == AckStatus.OK

        # Send block with wrong offset (should be 1024)
        transport.send(Command.data_block(offset=512, data=data[1024:]))
        response = transport.receive()

        assert response.status == AckStatus.BAD_COMMAND

    def test_crc_mismatch(self, transport):
        """Scenario: Detect CRC mismatch on finish."""
        from crispy_protocol.protocol import AckStatus, Command

        data = b"\x00" * 1024
        wrong_crc = 0xDEADBEEF

        transport.send(Command.start_update(bank=0, size=len(data), crc32=wrong_crc, version=1))
        assert transport.receive().status == AckStatus.OK

        transport.send(Command.data_block(offset=0, data=data))
        assert transport.receive().status == AckStatus.OK

        transport.send(Command.finish_update())
        response = transport.receive()

        assert response.status == AckStatus.CRC_ERROR


class TestReboot:
    """Feature: Reboot device."""

    def test_reboot_command(self, transport):
        """Scenario: Reboot the device."""
        from crispy_protocol.protocol import AckStatus, Command

        transport.send(Command.reboot())
        response = transport.receive()

        assert response.status == AckStatus.OK
        print("Reboot command sent successfully")

        # Device will disconnect after reboot
        time.sleep(2)


# Convenience function to run a full upload cycle
def upload_firmware(transport, firmware_data: bytes, bank: int, version: int) -> bool:
    """Helper to upload firmware to a bank."""
    from crispy_protocol.crc32 import crc32
    from crispy_protocol.protocol import AckStatus, Command

    size = len(firmware_data)
    checksum = crc32(firmware_data)

    transport.send(Command.start_update(bank=bank, size=size, crc32=checksum, version=version))
    if transport.receive().status != AckStatus.OK:
        return False

    offset = 0
    while offset < size:
        chunk = firmware_data[offset : offset + 1024]
        transport.send(Command.data_block(offset=offset, data=chunk))
        if transport.receive().status != AckStatus.OK:
            return False
        offset += len(chunk)

    transport.send(Command.finish_update())
    return transport.receive().status == AckStatus.OK


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--device", "/dev/ttyACM0"])

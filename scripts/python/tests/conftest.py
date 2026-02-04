# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""Pytest configuration for integration tests."""

import subprocess
import time
from pathlib import Path

import pytest

# Constants matching crispy-common/src/protocol.rs
RAM_UPDATE_FLAG_ADDR = 0x2003_BFF0
RAM_UPDATE_MAGIC = 0x0FDA_7E00
CHIP = "rp2040"


def pytest_addoption(parser):
    """Add custom command-line options."""
    parser.addoption(
        "--device",
        action="store",
        default=None,
        help="Serial port for the device (e.g., /dev/ttyACM0)",
    )
    parser.addoption(
        "--skip-build",
        action="store_true",
        default=False,
        help="Skip building firmware (use existing binaries)",
    )
    parser.addoption(
        "--skip-flash",
        action="store_true",
        default=False,
        help="Skip flashing device (assume already flashed)",
    )


def run_probe_rs(*args):
    """Run a probe-rs command and return (success, output)."""
    cmd = ["probe-rs"] + list(args)
    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.returncode == 0, result.stdout + result.stderr


def flash_elf(elf_path: Path) -> bool:
    """Flash an ELF file to the device via SWD."""
    print(f"Flashing {elf_path} via SWD...")
    success, output = run_probe_rs("download", "--chip", CHIP, str(elf_path))
    if not success:
        print(f"Flash failed: {output}")
    return success


def erase_flash() -> bool:
    """Erase the entire flash via SWD."""
    print("Erasing flash...")
    success, output = run_probe_rs("erase", "--chip", CHIP)
    if not success:
        print(f"Erase failed: {output}")
    return success


def reset_device() -> bool:
    """Reset the device via SWD."""
    success, _ = run_probe_rs("reset", "--chip", CHIP)
    return success


def enter_update_mode_via_swd() -> bool:
    """Enter bootloader update mode by writing RAM magic and resetting."""
    print("Entering update mode via SWD...")
    # Write RAM magic value
    success, output = run_probe_rs(
        "write", "--chip", CHIP, "b32",
        hex(RAM_UPDATE_FLAG_ADDR), hex(RAM_UPDATE_MAGIC)
    )
    if not success:
        print(f"Failed to write RAM magic: {output}")
        return False
    # Reset device
    time.sleep(0.1)
    success, output = run_probe_rs("reset", "--chip", CHIP)
    if not success:
        print(f"Failed to reset: {output}")
        return False
    # Wait for bootloader to initialize USB
    time.sleep(2.0)
    return True


def find_bootloader_port(timeout: float = 10.0) -> str:
    """Find the serial port for the Crispy Bootloader by USB ID."""
    import glob
    import os

    # USB IDs for the bootloader
    BOOTLOADER_VID = "2e8a"
    BOOTLOADER_PID = "000a"

    start = time.time()
    while time.time() - start < timeout:
        for port in glob.glob("/dev/ttyACM*"):
            # Check the USB vendor/product ID via sysfs
            try:
                # /dev/ttyACM0 -> /sys/class/tty/ttyACM0/device/../idVendor
                tty_name = os.path.basename(port)
                sys_path = f"/sys/class/tty/{tty_name}/device/.."

                with open(f"{sys_path}/idVendor", "r") as f:
                    vid = f.read().strip()
                with open(f"{sys_path}/idProduct", "r") as f:
                    pid = f.read().strip()

                if vid == BOOTLOADER_VID and pid == BOOTLOADER_PID:
                    return port
            except (FileNotFoundError, IOError):
                continue
        time.sleep(0.5)

    raise TimeoutError("Bootloader serial port not found")


@pytest.fixture(scope="session")
def device_port(request):
    """Get the device port from command line (optional override)."""
    return request.config.getoption("--device")


@pytest.fixture(scope="session")
def skip_build(request):
    """Check if build should be skipped."""
    return request.config.getoption("--skip-build")


@pytest.fixture(scope="session")
def skip_flash(request):
    """Check if flash should be skipped."""
    return request.config.getoption("--skip-flash")


@pytest.fixture(scope="session")
def project_root():
    """Get the project root directory."""
    return Path(__file__).parent.parent.parent.parent


@pytest.fixture(scope="session")
def flashed_device(project_root, skip_flash):
    """
    Ensure device has bootloader flashed.

    This fixture:
    1. Builds bootloader if necessary
    2. Flashes the bootloader ELF via SWD
    3. Resets the device

    Note: Firmware is NOT flashed here - it will be uploaded via USB
    protocol during tests to test the real update workflow.
    """
    if skip_flash:
        print("Skipping flash (--skip-flash)")
        return True

    target_dir = project_root / "target" / "thumbv6m-none-eabi" / "release"
    bootloader_elf = target_dir / "crispy-bootloader"

    # Build if necessary
    if not bootloader_elf.exists():
        print("Bootloader not found, building...")
        result = subprocess.run(
            [
                "cargo", "build", "--release",
                "-p", "crispy-bootloader",
                "-p", "crispy-fw-sample-rs",
                "--target", "thumbv6m-none-eabi",
            ],
            cwd=project_root,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            pytest.fail(f"Failed to build: {result.stderr}")

    if not bootloader_elf.exists():
        pytest.fail(f"Bootloader ELF not found: {bootloader_elf}")

    # Flash only bootloader (firmware will be uploaded via USB during tests)
    if not flash_elf(bootloader_elf):
        pytest.fail("Failed to flash bootloader")

    # Reset device - bootloader will enter update mode since no valid firmware
    reset_device()
    time.sleep(2.0)

    return True


@pytest.fixture(scope="session")
def device_in_update_mode(flashed_device):
    """
    Ensure device is in bootloader update mode.

    Uses SWD to write RAM magic flag and reset.
    """
    if not enter_update_mode_via_swd():
        pytest.fail("Failed to enter update mode via SWD")
    return True


@pytest.fixture
def transport(device_in_update_mode):
    """
    Create a transport connection to the device in update mode.

    This is function-scoped so each test that modifies bootloader state
    gets a fresh connection. The fixture resets the bootloader via SWD
    before creating the connection.
    """
    from crispy_protocol.transport import Transport

    # Reset bootloader to Idle state via SWD
    enter_update_mode_via_swd()

    # Find the bootloader port
    try:
        port = find_bootloader_port(timeout=5.0)
    except TimeoutError:
        pytest.fail("Bootloader serial port not found after reset")

    # Give device time to enumerate USB
    time.sleep(0.5)

    transport = Transport(port, timeout=5.0)
    yield transport
    transport.close()

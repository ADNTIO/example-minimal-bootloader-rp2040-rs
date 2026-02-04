#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>

"""
Firmware upload tool for crispy-bootloader via USB CDC.

Usage:
    python crispy_upload.py --port /dev/ttyACM0 status
    python crispy_upload.py --port /dev/ttyACM0 upload firmware.bin --bank 0 --version 1
    python crispy_upload.py --port /dev/ttyACM0 reboot

Requirements:
    pip install pyserial
"""

import argparse
import sys
from pathlib import Path

try:
    import serial
except ImportError:
    print("Error: pyserial not installed. Run: pip install pyserial")
    sys.exit(1)

from crispy_protocol import Transport, crc32
from crispy_protocol.transport import TransportError, UploadError


def cmd_status(transport: Transport):
    """Get bootloader status."""
    status = transport.get_status()

    print("Bootloader Status:")
    print(f"  Active bank: {status.active_bank} ({status.active_bank_name})")
    print(f"  Version A:   {status.version_a}")
    print(f"  Version B:   {status.version_b}")
    print(f"  State:       {status.state}")


def cmd_upload(transport: Transport, firmware_path: Path, bank: int, version: int):
    """Upload firmware to a bank."""
    firmware = firmware_path.read_bytes()
    size = len(firmware)
    checksum = crc32(firmware)

    print(f"Firmware: {firmware_path} ({size} bytes, CRC32: 0x{checksum:08x})")
    print(f"Target:   Bank {bank} ({'A' if bank == 0 else 'B'})")
    print(f"Version:  {version}")
    print()

    print("Starting update... ", end="", flush=True)

    def progress(sent: int, total: int):
        pct = sent * 100 // total
        print(f"\rUploading: {pct:3d}% ({sent}/{total} bytes)", end="", flush=True)

    try:
        # Start is handled inside upload_firmware, show OK after first ack
        resp = transport.start_update(bank, size, checksum, version)
        if not resp.is_ok:
            print(f"FAILED: {resp.status}")
            return False
        print("OK")

        # Send data blocks with progress
        offset = 0
        chunk_size = 1024
        while offset < size:
            chunk = firmware[offset:offset + chunk_size]
            resp = transport.send_data_block(offset, chunk)

            if not resp.is_ok:
                print(f"\nDataBlock failed at offset {offset}: {resp.status}")
                return False

            offset += len(chunk)
            progress(offset, size)

        print("\rUploading: 100% - Complete!          ")

        # Finish
        print("Finalizing... ", end="", flush=True)
        resp = transport.finish_update()
        if not resp.is_ok:
            print(f"FAILED: {resp.status}")
            return False
        print("OK")

    except UploadError as e:
        print(f"FAILED: {e}")
        return False

    print()
    print("Firmware uploaded successfully!")
    print(f"Use: python {sys.argv[0]} --port {transport.port} reboot")
    return True


def cmd_reboot(transport: Transport):
    """Reboot the device."""
    print("Rebooting device... ", end="", flush=True)
    resp = transport.reboot()

    if resp.is_ok:
        print("OK")
    else:
        print(f"FAILED: {resp.status}")


def main():
    parser = argparse.ArgumentParser(
        description="Firmware upload tool for crispy-bootloader"
    )
    parser.add_argument(
        "--port", "-p",
        required=True,
        help="Serial port (e.g., /dev/ttyACM0)"
    )

    subparsers = parser.add_subparsers(dest="command", required=True)

    # status command
    subparsers.add_parser("status", help="Get bootloader status")

    # upload command
    upload_parser = subparsers.add_parser("upload", help="Upload firmware to a bank")
    upload_parser.add_argument("file", type=Path, help="Firmware binary file")
    upload_parser.add_argument("--bank", "-b", type=int, default=0, choices=[0, 1],
                               help="Target bank (0=A, 1=B)")
    upload_parser.add_argument("--version", "-v", type=int, default=1,
                               help="Firmware version number")

    # reboot command
    subparsers.add_parser("reboot", help="Reboot the device")

    args = parser.parse_args()

    try:
        transport = Transport(args.port)
    except serial.SerialException as e:
        print(f"Error opening {args.port}: {e}")
        sys.exit(1)

    try:
        if args.command == "status":
            cmd_status(transport)
        elif args.command == "upload":
            if not args.file.exists():
                print(f"Error: File not found: {args.file}")
                sys.exit(1)
            cmd_upload(transport, args.file, args.bank, args.version)
        elif args.command == "reboot":
            cmd_reboot(transport)
    except TransportError as e:
        print(f"Error: {e}")
        sys.exit(1)
    finally:
        transport.close()


if __name__ == "__main__":
    main()

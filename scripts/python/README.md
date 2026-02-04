# Crispy Bootloader Python Tools

Python client library and CLI tool for communicating with the crispy-bootloader via USB CDC.

## Requirements

- Python 3.8+
- pyserial

```bash
pip install pyserial
```

## CLI Usage

The `crispy_upload.py` script provides a command-line interface for common operations.

### Get Bootloader Status

```bash
python crispy_upload.py --port /dev/ttyACM0 status
```

Output:
```
Bootloader Status:
  Active bank: 0 (A)
  Version A:   5
  Version B:   4
  State:       UPDATE_MODE
```

### Upload Firmware

```bash
python crispy_upload.py --port /dev/ttyACM0 upload firmware.bin --bank 0 --version 5
```

Options:
- `--bank`, `-b`: Target bank (0=A, 1=B, default: 0)
- `--version`, `-v`: Firmware version number (default: 1)

Output:
```
Firmware: firmware.bin (31916 bytes, CRC32: 0x2b48b782)
Target:   Bank 0 (A)
Version:  5

Starting update... OK
Uploading: 100% - Complete!
Finalizing... OK

Firmware uploaded successfully!
```

### Reboot Device

```bash
python crispy_upload.py --port /dev/ttyACM0 reboot
```

## Library Usage

The `crispy_protocol` package can be used programmatically in your own scripts.

### Basic Example

```python
from crispy_protocol import Transport

with Transport("/dev/ttyACM0") as t:
    # Get bootloader status
    status = t.get_status()
    print(f"Active bank: {status.active_bank_name}")
    print(f"Version A: {status.version_a}")
    print(f"Version B: {status.version_b}")
    print(f"State: {status.state}")

    # Upload firmware with progress callback
    def progress(sent, total):
        print(f"\r{sent}/{total} bytes", end="")

    t.upload_firmware(
        firmware=open("firmware.bin", "rb").read(),
        bank=0,
        version=5,
        progress_callback=progress
    )
    print("\nUpload complete!")

    # Reboot to new firmware
    t.reboot()
```

### Low-Level API

```python
from crispy_protocol import Transport, AckStatus

with Transport("/dev/ttyACM0") as t:
    # Start update manually
    firmware = open("firmware.bin", "rb").read()
    from crispy_protocol import crc32

    resp = t.start_update(
        bank=0,
        size=len(firmware),
        crc=crc32(firmware),
        version=5
    )
    if not resp.is_ok:
        print(f"Start failed: {resp.status}")

    # Send data blocks
    offset = 0
    while offset < len(firmware):
        chunk = firmware[offset:offset+1024]
        resp = t.send_data_block(offset, chunk)
        if not resp.is_ok:
            print(f"Block failed: {resp.status}")
            break
        offset += len(chunk)

    # Finish update
    resp = t.finish_update()
    if resp.is_ok:
        print("Update complete!")
    elif resp.status == AckStatus.CRC_ERROR:
        print("CRC verification failed")
```

### Using the CRC32 Function

```python
from crispy_protocol import crc32

data = open("firmware.bin", "rb").read()
checksum = crc32(data)
print(f"CRC32: 0x{checksum:08x}")
```

## Protocol Details

The bootloader uses a binary protocol with:
- **Framing**: COBS (Consistent Overhead Byte Stuffing) with 0x00 delimiter
- **Serialization**: Postcard format (Rust's serde-based binary format)
- **Integers**: Variable-length encoding (LEB128/varint)
- **Checksum**: CRC-32 (ISO HDLC polynomial)

### Commands

| Command | Description |
|---------|-------------|
| `GetStatus` | Get bootloader state and bank information |
| `StartUpdate(bank, size, crc32, version)` | Begin firmware update |
| `DataBlock(offset, data)` | Send firmware data chunk (max 1024 bytes) |
| `FinishUpdate` | Complete update and verify CRC |
| `Reboot` | Reboot the device |

### Response Status Codes

| Status | Description |
|--------|-------------|
| `OK` | Operation successful |
| `CRC_ERROR` | CRC verification failed |
| `FLASH_ERROR` | Flash write/erase error |
| `BAD_COMMAND` | Invalid command |
| `BAD_STATE` | Command not valid in current state |
| `BANK_INVALID` | Invalid bank number |

## Entering Bootloader Mode

The device must be in bootloader mode to accept commands. Methods to enter bootloader mode:

1. **From firmware**: Send `bootload` command via USB serial
   ```bash
   echo "bootload" > /dev/ttyACM0
   ```

2. **Hardware trigger**: Hold GP2 low during reset

3. **Auto-entry**: The bootloader stays in update mode if the firmware is not confirmed or after 3 failed boot attempts

## Package Structure

```
crispy_protocol/
    __init__.py      # Package exports
    cobs.py          # COBS encode/decode
    crc32.py         # CRC-32 calculation
    varint.py        # LEB128 varint encoding
    protocol.py      # Command/Response definitions
    transport.py     # Serial transport layer
```

## License

MIT License - Copyright (c) 2026 ADNT Sarl

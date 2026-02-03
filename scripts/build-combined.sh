#!/usr/bin/env bash
# Build combined binary (bootloader + firmware bank A) for RP2040
# Usage: ./scripts/build-combined.sh [profile]
#   profile: release (default) or release-debug
set -e

PROFILE="${1:-release}"
TARGET="thumbv6m-none-eabi"
OUTPUT_DIR="target/${TARGET}/${PROFILE}"

BOOTLOADER_ELF="${OUTPUT_DIR}/crispy-bootloader"
FW_ELF="${OUTPUT_DIR}/crispy-fw-sample"

# Check ELFs exist
for f in "$BOOTLOADER_ELF" "$FW_ELF"; do
  if [ ! -f "$f" ]; then
    echo "Error: $f not found. Run: cargo build --profile ${PROFILE} -p crispy-bootloader -p crispy-fw-sample"
    exit 1
  fi
done

# Convert ELF to BIN
arm-none-eabi-objcopy -O binary "$BOOTLOADER_ELF" "${OUTPUT_DIR}/bootloader.bin"
arm-none-eabi-objcopy -O binary "$FW_ELF" "${OUTPUT_DIR}/fw1.bin"

# Firmware starts at offset 0x10000 (65536 bytes) in flash (sector-aligned after 64KB bootloader)
FW_OFFSET=65536
FW_SIZE=$(stat -c%s "${OUTPUT_DIR}/fw1.bin")
COMBINED_SIZE=$(( (FW_OFFSET + FW_SIZE + 255) / 256 * 256 ))

# Create combined binary
dd if=/dev/zero of="${OUTPUT_DIR}/combined.bin" bs=1 count="$COMBINED_SIZE" 2>/dev/null
dd if="${OUTPUT_DIR}/bootloader.bin" of="${OUTPUT_DIR}/combined.bin" bs=1 seek=0 conv=notrunc 2>/dev/null
dd if="${OUTPUT_DIR}/fw1.bin" of="${OUTPUT_DIR}/combined.bin" bs=1 seek="$FW_OFFSET" conv=notrunc 2>/dev/null

echo "Combined binary: ${OUTPUT_DIR}/combined.bin ($(stat -c%s "${OUTPUT_DIR}/combined.bin") bytes)"

# Create UF2
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
python3 "${SCRIPT_DIR}/bin2uf2.py" "${OUTPUT_DIR}/combined.bin" "${OUTPUT_DIR}/combined.uf2" 0x10000000

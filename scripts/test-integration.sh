#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 ADNT Sarl <info@adnt.io>
#
# Integration test script for crispy-bootloader
#
# Usage:
#   ./scripts/test-integration.sh [OPTIONS]
#
# Options:
#   --device PORT    Serial port (default: auto-detect /dev/ttyACM*)
#   --skip-build     Skip building (use existing binaries)
#   --skip-flash     Skip flashing (device already in update mode)
#   --verbose        Enable verbose output
#   --help           Show this help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default options
DEVICE=""
SKIP_BUILD=false
SKIP_FLASH=false
VERBOSE=false

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

usage() {
    head -20 "$0" | tail -15 | sed 's/^# //' | sed 's/^#//'
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-flash)
            SKIP_FLASH=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Auto-detect device
detect_device() {
    if [[ -n "$DEVICE" ]]; then
        return
    fi

    local devices
    devices=$(ls /dev/ttyACM* 2>/dev/null || true)

    if [[ -z "$devices" ]]; then
        log_error "No USB CDC device found (/dev/ttyACM*)"
        log_info "Make sure the device is connected and in update mode (GP2 held low during reset)"
        exit 1
    fi

    DEVICE=$(echo "$devices" | head -1)
    log_info "Auto-detected device: $DEVICE"
}

# Build step
build_artifacts() {
    if $SKIP_BUILD; then
        log_warn "Skipping build (--skip-build)"
        return
    fi

    log_info "Building bootloader..."
    cargo build --release -p crispy-bootloader --target thumbv6m-none-eabi
    log_ok "Bootloader built"

    log_info "Building firmware..."
    cargo build --release -p crispy-fw-sample --target thumbv6m-none-eabi
    log_ok "Firmware built"

    log_info "Creating firmware binary..."
    arm-none-eabi-objcopy -O binary \
        "$PROJECT_ROOT/target/thumbv6m-none-eabi/release/crispy-fw-sample" \
        "$PROJECT_ROOT/target/firmware.bin"

    local size
    size=$(stat -c%s "$PROJECT_ROOT/target/firmware.bin")
    log_ok "Firmware binary created: $size bytes"

    log_info "Creating combined binary..."
    if [[ -x "$PROJECT_ROOT/scripts/build-combined.sh" ]]; then
        "$PROJECT_ROOT/scripts/build-combined.sh"
        log_ok "Combined binary created"
    else
        log_warn "build-combined.sh not found, skipping combined binary"
    fi
}

# Flash step
flash_device() {
    if $SKIP_FLASH; then
        log_warn "Skipping flash (--skip-flash)"
        return
    fi

    local uf2_file="$PROJECT_ROOT/target/thumbv6m-none-eabi/release/combined.uf2"

    if [[ ! -f "$uf2_file" ]]; then
        log_warn "Combined UF2 not found, looking for bootloader UF2..."
        uf2_file="$PROJECT_ROOT/target/thumbv6m-none-eabi/release/crispy-bootloader.uf2"
    fi

    if [[ ! -f "$uf2_file" ]]; then
        log_error "No UF2 file found to flash"
        log_info "Run without --skip-build to create binaries"
        exit 1
    fi

    # Check for picotool
    if ! command -v picotool &>/dev/null; then
        log_warn "picotool not found"
        log_info "Please flash manually:"
        log_info "  1. Hold BOOTSEL and reset the device"
        log_info "  2. Copy $uf2_file to the RPI-RP2 drive"
        log_info "  3. Re-run with --skip-flash"
        exit 1
    fi

    log_info "Flashing device with picotool..."
    picotool load -f "$uf2_file"
    log_ok "Device flashed"

    log_info "Waiting for device to boot..."
    sleep 2
}

# Wait for device to enter update mode
wait_for_update_mode() {
    log_info "Waiting for device to appear..."

    local attempts=0
    local max_attempts=30

    while [[ $attempts -lt $max_attempts ]]; do
        if [[ -e "$DEVICE" ]]; then
            log_ok "Device found: $DEVICE"
            return
        fi
        sleep 1
        ((attempts++))
    done

    log_error "Device did not appear after ${max_attempts}s"
    log_info "Make sure to hold GP2 low during reset to enter update mode"
    exit 1
}

# Run Python tests
run_tests() {
    log_info "Running integration tests..."

    cd "$PROJECT_ROOT/scripts/python"

    local pytest_args=("-v" "--device" "$DEVICE")

    if $SKIP_BUILD; then
        pytest_args+=("--skip-build")
    fi

    if $VERBOSE; then
        pytest_args+=("-s")
    fi

    # Activate venv if exists
    if [[ -d ".venv" ]]; then
        source .venv/bin/activate
    fi

    python -m pytest tests/test_integration.py "${pytest_args[@]}"
}

# Quick status check
check_status() {
    log_info "Checking bootloader status..."

    cd "$PROJECT_ROOT/scripts/python"

    if [[ -d ".venv" ]]; then
        source .venv/bin/activate
    fi

    python -c "
from crispy_protocol.transport import Transport
from crispy_protocol.protocol import Command

t = Transport('$DEVICE', timeout=2.0)
t.send(Command.get_status())
r = t.receive()
print(f'Active bank: {r.active_bank}')
print(f'Version A:   {r.version_a}')
print(f'Version B:   {r.version_b}')
print(f'State:       {r.state}')
"
}

# Main
main() {
    log_info "Crispy Bootloader Integration Test"
    echo

    cd "$PROJECT_ROOT"

    build_artifacts
    echo

    if ! $SKIP_FLASH; then
        flash_device
    fi

    detect_device
    wait_for_update_mode
    echo

    check_status
    echo

    run_tests
    echo

    log_ok "All integration tests passed!"
}

main "$@"

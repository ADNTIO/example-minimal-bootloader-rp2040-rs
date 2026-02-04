// SPDX-License-Identifier: MIT
// Crispy Bootloader - BootData structure and operations

#pragma once

#include "protocol.h"

namespace crispy {

// BootData structure (must match crispy-common, 32 bytes)
struct __attribute__((packed)) BootData {
    uint32_t magic;
    uint8_t  active_bank;
    uint8_t  confirmed;
    uint8_t  boot_attempts;
    uint8_t  _reserved0;
    uint32_t version_a;
    uint32_t version_b;
    uint32_t crc_a;
    uint32_t crc_b;
    uint32_t size_a;
    uint32_t size_b;

    bool is_valid() const { return magic == BOOT_DATA_MAGIC; }
    const char* bank_name() const { return active_bank == 0 ? "A" : "B"; }
};
static_assert(sizeof(BootData) == 32, "BootData must be 32 bytes");

// Read BootData from flash
BootData read_boot_data();

// Confirm boot to bootloader (write confirmed=1, boot_attempts=0)
void confirm_boot();

// Reboot to bootloader update mode
[[noreturn]] void reboot_to_bootloader();

// Reboot normally
[[noreturn]] void reboot();

} // namespace crispy

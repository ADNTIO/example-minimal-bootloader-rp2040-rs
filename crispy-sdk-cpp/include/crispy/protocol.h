// SPDX-License-Identifier: MIT
// Crispy Bootloader Protocol - must match crispy-common (Rust)

#pragma once

#include <cstdint>

namespace crispy {

// Flash layout
constexpr uint32_t FLASH_BASE_ADDR      = 0x10000000;
constexpr uint32_t FW_A_ADDR            = 0x10010000;
constexpr uint32_t FW_B_ADDR            = 0x100D0000;
constexpr uint32_t BOOT_DATA_ADDR       = 0x10190000;

constexpr uint32_t FW_BANK_SIZE         = 768 * 1024;  // 768KB per bank
constexpr uint32_t BOOT_DATA_MAGIC      = 0xB007DA7A;

// RAM flags for bootloader communication
constexpr uint32_t RAM_UPDATE_FLAG_ADDR = 0x2003BFF0;
constexpr uint32_t RAM_UPDATE_MAGIC     = 0x0FDA7E00;

// Hardware
constexpr uint32_t LED_PIN = 25;

} // namespace crispy

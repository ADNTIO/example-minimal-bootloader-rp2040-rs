// SPDX-License-Identifier: MIT
// Crispy Bootloader - BootData operations

#include "crispy/boot_data.h"
#include "pico/stdlib.h"
#include "hardware/flash.h"
#include "hardware/sync.h"
#include "hardware/watchdog.h"
#include <cstring>
#include <cstdio>

namespace crispy {

BootData read_boot_data() {
    const auto* bd = reinterpret_cast<const BootData*>(BOOT_DATA_ADDR);
    return *bd;
}

void confirm_boot() {
    BootData bd = read_boot_data();

    if (!bd.is_valid()) {
        printf("BootData invalid, skipping confirmation\r\n");
        return;
    }
    if (bd.confirmed == 1) {
        printf("Boot already confirmed\r\n");
        return;
    }

    printf("Confirming boot (bank=%d)...\r\n", bd.active_bank);

    bd.confirmed = 1;
    bd.boot_attempts = 0;

    uint32_t offset = BOOT_DATA_ADDR - FLASH_BASE_ADDR;

    // Pad to FLASH_PAGE_SIZE (256 bytes)
    uint8_t page[FLASH_PAGE_SIZE];
    memset(page, 0xFF, sizeof(page));
    memcpy(page, &bd, sizeof(bd));

    // Disable interrupts during flash operations
    uint32_t ints = save_and_disable_interrupts();

    // Erase sector (4KB) and program page
    flash_range_erase(offset, FLASH_SECTOR_SIZE);
    flash_range_program(offset, page, sizeof(page));

    restore_interrupts(ints);

    printf("Boot confirmed successfully\r\n");
}

void reboot_to_bootloader() {
    printf("Rebooting to bootloader update mode...\r\n");
    sleep_ms(100);

    // Write magic to RAM flag
    *reinterpret_cast<volatile uint32_t*>(RAM_UPDATE_FLAG_ADDR) = RAM_UPDATE_MAGIC;

    // Trigger watchdog reset
    watchdog_reboot(0, 0, 0);
    while (true) tight_loop_contents();
}

void reboot() {
    printf("Rebooting...\r\n");
    sleep_ms(100);
    watchdog_reboot(0, 0, 0);
    while (true) tight_loop_contents();
}

} // namespace crispy

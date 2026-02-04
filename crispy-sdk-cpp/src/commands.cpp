// SPDX-License-Identifier: MIT
// Crispy Bootloader - Command processing

#include "crispy/commands.h"
#include "crispy/boot_data.h"
#include <cstring>
#include <cstdio>

namespace crispy {

bool process_command(const char* line) {
    // Trim leading whitespace
    while (*line == ' ' || *line == '\t') line++;

    if (strcmp(line, "help") == 0 || strcmp(line, "?") == 0) {
        printf("Available commands:\r\n");
        printf("  help     - Show this help\r\n");
        printf("  status   - Show boot status\r\n");
        printf("  bootload - Reboot to bootloader update mode\r\n");
        printf("  reboot   - Reboot normally\r\n");
    }
    else if (strcmp(line, "status") == 0) {
        BootData bd = read_boot_data();
        if (bd.is_valid()) {
            printf("Boot status:\r\n");
            printf("  Bank: %d (%s)\r\n", bd.active_bank, bd.bank_name());
            printf("  Confirmed: %d\r\n", bd.confirmed);
            printf("  Attempts: %d\r\n", bd.boot_attempts);
            printf("  Version A: %lu\r\n", bd.version_a);
            printf("  Version B: %lu\r\n", bd.version_b);
        } else {
            printf("BootData: invalid\r\n");
        }
    }
    else if (strcmp(line, "bootload") == 0) {
        printf("Rebooting to bootloader...\r\n");
        return true;
    }
    else if (strcmp(line, "reboot") == 0) {
        reboot();
    }
    else if (line[0] != '\0') {
        printf("Unknown command. Type 'help' for available commands.\r\n");
    }

    return false;
}

void print_welcome() {
    printf("\r\n=== Crispy C++ Firmware ===\r\n");
    printf("Type 'help' for available commands.\r\n");
}

void print_prompt() {
    printf("> ");
}

} // namespace crispy

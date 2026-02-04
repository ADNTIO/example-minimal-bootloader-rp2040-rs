// SPDX-License-Identifier: MIT
// Crispy Bootloader - Command processing

#pragma once

namespace crispy {

// Process a command line. Returns true if should reboot to bootloader.
bool process_command(const char* line);

// Print welcome message
void print_welcome();

// Print command prompt
void print_prompt();

} // namespace crispy

// SPDX-License-Identifier: MIT
// Minimal bare-metal C++ firmware for Crispy Bootloader
// No Pico SDK runtime - just direct register access

#include <stdint.h>

// RP2040 Register definitions
#define SIO_BASE            0xd0000000
#define GPIO_OUT_SET        (*(volatile uint32_t*)(SIO_BASE + 0x014))
#define GPIO_OUT_CLR        (*(volatile uint32_t*)(SIO_BASE + 0x018))
#define GPIO_OE_SET         (*(volatile uint32_t*)(SIO_BASE + 0x024))

#define IO_BANK0_BASE       0x40014000
#define GPIO25_CTRL         (*(volatile uint32_t*)(IO_BANK0_BASE + 0x0cc))

#define RESETS_BASE         0x4000c000
#define RESETS_RESET        (*(volatile uint32_t*)(RESETS_BASE + 0x00))
#define RESETS_RESET_DONE   (*(volatile uint32_t*)(RESETS_BASE + 0x08))

#define LED_PIN 25

// Simple busy-wait delay
static void delay(uint32_t count) {
    for (volatile uint32_t i = 0; i < count; i++) {
        __asm volatile("nop");
    }
}

// Main function - called by crt0 after BSS init
extern "C" int main() {
    // GPIO25 should already be out of reset (bootloader did this)
    // Just configure it for SIO function and output

    GPIO25_CTRL = 5;        // Function 5 = SIO
    GPIO_OE_SET = (1 << LED_PIN);  // Enable output

    // Blink forever
    while (true) {
        GPIO_OUT_SET = (1 << LED_PIN);
        delay(2000000);  // ~200ms at 125MHz
        GPIO_OUT_CLR = (1 << LED_PIN);
        delay(2000000);
    }

    return 0;
}

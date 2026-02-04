// SPDX-License-Identifier: MIT
// C++ firmware for Crispy Bootloader using Pico SDK

#include <crispy/crispy.h>

#include "pico/stdlib.h"
#include "hardware/gpio.h"
#include <cstdio>

using namespace crispy;

namespace {
    char cmd_buf[64];
    size_t cmd_pos = 0;
}

int main() {
    stdio_init_all();

    // Initialize LED
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);

    // Quick blink to signal firmware alive
    for (int i = 0; i < 5; i++) {
        gpio_put(LED_PIN, 1);
        sleep_ms(100);
        gpio_put(LED_PIN, 0);
        sleep_ms(100);
    }

    // Confirm boot to bootloader
    confirm_boot();

    print_welcome();
    print_prompt();

    uint32_t last_blink = 0;
    bool led_state = false;

    while (true) {
        // Read USB CDC input
        int c = getchar_timeout_us(0);
        if (c != PICO_ERROR_TIMEOUT) {
            char ch = static_cast<char>(c);
            putchar(ch);

            if (ch == '\r' || ch == '\n') {
                printf("\r\n");
                if (cmd_pos > 0) {
                    cmd_buf[cmd_pos] = '\0';
                    if (process_command(cmd_buf)) {
                        sleep_ms(100);
                        reboot_to_bootloader();
                    }
                    cmd_pos = 0;
                }
                print_prompt();
            }
            else if (ch == 0x7F || ch == 0x08) {
                if (cmd_pos > 0) {
                    cmd_pos--;
                    printf("\b \b");
                }
            }
            else if (cmd_pos < sizeof(cmd_buf) - 1) {
                cmd_buf[cmd_pos++] = ch;
            }
        }

        // Slow blink LED (toggle every 500ms)
        uint32_t now = to_ms_since_boot(get_absolute_time());
        if (now - last_blink >= 500) {
            last_blink = now;
            led_state = !led_state;
            gpio_put(LED_PIN, led_state);
        }
    }
}

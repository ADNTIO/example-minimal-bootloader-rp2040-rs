# Crispy SDK for C++

SDK for developing C++ firmwares compatible with the Crispy Bootloader on RP2040.

## Features

- **Boot confirmation**: Automatically confirms boot to prevent rollback
- **USB CDC**: Serial communication via USB
- **Built-in commands**: `help`, `status`, `bootload`, `reboot`
- **Linker script**: RAM configuration for execution from the bootloader

## Prerequisites

- [Pico SDK](https://github.com/raspberrypi/pico-sdk)
- CMake 3.13+
- ARM GCC toolchain

## Usage

### 1. Add the SDK to your project

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.13)

# Pico SDK setup
include(pico_sdk_import.cmake)
project(my_firmware C CXX ASM)
pico_sdk_init()

# Include Crispy SDK
add_subdirectory(path/to/crispy-sdk-cpp)

# Your firmware
add_executable(my_firmware main.cpp)
target_link_libraries(my_firmware pico_stdlib crispy_sdk)

# Configure for Crispy bootloader
crispy_configure_firmware(my_firmware)
```

### 2. Minimal code

```cpp
#include <crispy/crispy.h>
#include "pico/stdlib.h"

using namespace crispy;

int main() {
    stdio_init_all();

    // Confirm boot to the bootloader
    confirm_boot();

    // Your code...
    while (true) {
        // ...
    }
}
```

## API

### Protocol (`crispy/protocol.h`)

```cpp
namespace crispy {
    constexpr uint32_t FLASH_BASE_ADDR      = 0x10000000;
    constexpr uint32_t BOOT_DATA_ADDR       = 0x10190000;
    constexpr uint32_t BOOT_DATA_MAGIC      = 0xB007DA7A;
    constexpr uint32_t RAM_UPDATE_FLAG_ADDR = 0x2003BFF0;
    constexpr uint32_t RAM_UPDATE_MAGIC     = 0x0FDA7E00;
    constexpr uint32_t LED_PIN              = 25;
}
```

### BootData (`crispy/boot_data.h`)

```cpp
namespace crispy {
    struct BootData {
        uint32_t magic;
        uint8_t  active_bank;     // 0 = A, 1 = B
        uint8_t  confirmed;       // 1 = boot confirmed
        uint8_t  boot_attempts;   // Rollback after 3 attempts
        // ...

        bool is_valid() const;
        const char* bank_name() const;  // "A" or "B"
    };

    BootData read_boot_data();
    void confirm_boot();
    [[noreturn]] void reboot_to_bootloader();
    [[noreturn]] void reboot();
}
```

### Commands (`crispy/commands.h`)

```cpp
namespace crispy {
    bool process_command(const char* line);  // true = reboot to bootloader
    void print_welcome();
    void print_prompt();
}
```

## Available commands

| Command    | Description                              |
|------------|------------------------------------------|
| `help`     | Display help                             |
| `status`   | Display boot status (bank, confirmed)    |
| `bootload` | Reboot to update mode                    |
| `reboot`   | Reboot normally                          |

## File structure

```
crispy-sdk-cpp/
├── CMakeLists.txt
├── README.md
├── include/crispy/
│   ├── crispy.h          # Main include
│   ├── protocol.h        # Protocol constants
│   ├── boot_data.h       # BootData structure
│   └── commands.h        # Command processing
├── src/
│   ├── boot_data.cpp
│   └── commands.cpp
└── linker/
    └── memmap_crispy.ld  # RAM linker script
```

## Linker Script

The `memmap_crispy.ld` file configures the firmware for:

- **RAM execution** starting at `0x20000000`
- **Vector table at offset 0** (required by the bootloader)
- **192KB max** for the firmware

The bootloader copies the firmware from flash to RAM before executing it.

## Uploading firmware

```bash
# Enter update mode
make update-mode

# Or via serial command
echo "bootload" > /dev/ttyACM1

# Upload the firmware
crispy-upload -p /dev/ttyACM1 upload build/my_firmware.bin
crispy-upload -p /dev/ttyACM1 reboot
```

## Compatibility

- RP2040 (Raspberry Pi Pico)
- Pico SDK 1.5+
- Crispy Bootloader

## License

MIT

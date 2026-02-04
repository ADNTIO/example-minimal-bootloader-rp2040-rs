# Crispy SDK for C++

SDK pour développer des firmwares C++ compatibles avec le Crispy Bootloader sur RP2040.

## Fonctionnalités

- **Boot confirmation** : Confirme automatiquement le boot pour éviter le rollback
- **USB CDC** : Communication série via USB
- **Commandes intégrées** : `help`, `status`, `bootload`, `reboot`
- **Linker script** : Configuration RAM pour exécution depuis le bootloader

## Prérequis

- [Pico SDK](https://github.com/raspberrypi/pico-sdk)
- CMake 3.13+
- ARM GCC toolchain

## Utilisation

### 1. Ajouter le SDK à votre projet

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.13)

# Pico SDK setup
include(pico_sdk_import.cmake)
project(my_firmware C CXX ASM)
pico_sdk_init()

# Inclure Crispy SDK
add_subdirectory(path/to/crispy-sdk-cpp)

# Votre firmware
add_executable(my_firmware main.cpp)
target_link_libraries(my_firmware pico_stdlib crispy_sdk)

# Configurer pour Crispy bootloader
crispy_configure_firmware(my_firmware)
```

### 2. Code minimal

```cpp
#include <crispy/crispy.h>
#include "pico/stdlib.h"

using namespace crispy;

int main() {
    stdio_init_all();

    // Confirmer le boot au bootloader
    confirm_boot();

    // Votre code...
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
        uint8_t  confirmed;       // 1 = boot confirmé
        uint8_t  boot_attempts;   // Rollback après 3 tentatives
        // ...

        bool is_valid() const;
        const char* bank_name() const;  // "A" ou "B"
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

## Commandes disponibles

| Commande   | Description                              |
|------------|------------------------------------------|
| `help`     | Affiche l'aide                           |
| `status`   | Affiche l'état du boot (bank, confirmed) |
| `bootload` | Redémarre en mode mise à jour            |
| `reboot`   | Redémarre normalement                    |

## Structure des fichiers

```
crispy-sdk-cpp/
├── CMakeLists.txt
├── README.md
├── include/crispy/
│   ├── crispy.h          # Include principal
│   ├── protocol.h        # Constantes du protocole
│   ├── boot_data.h       # Structure BootData
│   └── commands.h        # Traitement des commandes
├── src/
│   ├── boot_data.cpp
│   └── commands.cpp
└── linker/
    └── memmap_crispy.ld  # Linker script RAM
```

## Linker Script

Le fichier `memmap_crispy.ld` configure le firmware pour :

- **Exécution en RAM** à partir de `0x20000000`
- **Vector table à offset 0** (requis par le bootloader)
- **192KB max** pour le firmware

Le bootloader copie le firmware depuis la flash vers la RAM avant de l'exécuter.

## Upload du firmware

```bash
# Mettre en mode update
make update-mode

# Ou via commande série
echo "bootload" > /dev/ttyACM1

# Uploader le firmware
crispy-upload -p /dev/ttyACM1 upload build/my_firmware.bin
crispy-upload -p /dev/ttyACM1 reboot
```

## Compatibilité

- RP2040 (Raspberry Pi Pico)
- Pico SDK 1.5+
- Crispy Bootloader

## Licence

MIT

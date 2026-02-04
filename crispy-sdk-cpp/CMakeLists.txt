# SPDX-License-Identifier: MIT
# Crispy SDK for C++ - Bootloader support library
#
# Usage in your project:
#   add_subdirectory(path/to/crispy-sdk-cpp)
#   target_link_libraries(your_target crispy_sdk)
#   crispy_configure_firmware(your_target)

cmake_minimum_required(VERSION 3.13)
project(crispy_sdk VERSION 0.2.0)

# Interface library (header-only + sources for linking)
add_library(crispy_sdk INTERFACE)

target_include_directories(crispy_sdk INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_sources(crispy_sdk INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/boot_data.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/commands.cpp
)

# Function to configure a target for Crispy bootloader
function(crispy_configure_firmware TARGET)
    # Link required Pico SDK libraries
    target_link_libraries(${TARGET}
        hardware_flash
        hardware_sync
        hardware_watchdog
    )

    # Enable USB CDC stdio
    pico_enable_stdio_usb(${TARGET} 1)
    pico_enable_stdio_uart(${TARGET} 0)

    # Set RAM execution flags
    target_compile_definitions(${TARGET} PRIVATE
        PICO_NO_FLASH=1
    )

    # Use Crispy linker script
    pico_set_linker_script(${TARGET} ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/linker/memmap_crispy.ld)

    # Generate BIN file for upload
    find_program(OBJCOPY arm-none-eabi-objcopy)
    if(OBJCOPY)
        add_custom_command(TARGET ${TARGET} POST_BUILD
            COMMAND ${OBJCOPY} -O binary $<TARGET_FILE:${TARGET}> ${CMAKE_CURRENT_BINARY_DIR}/${TARGET}.bin
            COMMENT "Generating ${TARGET}.bin for Crispy bootloader"
        )
    endif()
endfunction()

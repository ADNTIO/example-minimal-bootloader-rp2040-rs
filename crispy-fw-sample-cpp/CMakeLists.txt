# SPDX-License-Identifier: MIT
# C++ Firmware Sample for Crispy Bootloader

cmake_minimum_required(VERSION 3.13)

# Auto-download Pico SDK if PICO_SDK_PATH is not set
if(NOT DEFINED ENV{PICO_SDK_PATH} AND NOT PICO_SDK_PATH)
    set(PICO_SDK_FETCH_FROM_GIT ON)
    set(PICOTOOL_FORCE_FETCH_FROM_GIT ON)
endif()

include(pico_sdk_import.cmake)

project(crispy-fw-sample-cpp C CXX ASM)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

pico_sdk_init()

# Include Crispy SDK
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../crispy-sdk-cpp crispy-sdk-cpp)

# Firmware executable
add_executable(crispy-fw-sample-cpp
    main.cpp
)

# Link Pico stdlib and Crispy SDK
target_link_libraries(crispy-fw-sample-cpp
    pico_stdlib
    crispy_sdk
)

# Configure for Crispy bootloader (USB CDC, linker script, BIN generation)
crispy_configure_firmware(crispy-fw-sample-cpp)

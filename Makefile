# Crispy RP2040 Bootloader - Build shortcuts

EMBEDDED_TARGET := thumbv6m-none-eabi
CHIP := RP2040

.PHONY: all embedded host bootloader firmware upload clean clippy test
.PHONY: flash-bootloader run-bootloader
.PHONY: update-mode reset

# Build everything
all: embedded host

# Build embedded packages (bootloader + firmware)
embedded:
	cargo build --release -p crispy-bootloader -p crispy-fw-sample --target $(EMBEDDED_TARGET)

# Build host upload tool
host:
	cargo build --release -p crispy-upload

# Individual targets
bootloader:
	cargo build --release -p crispy-bootloader --target $(EMBEDDED_TARGET)

firmware:
	cargo build --release -p crispy-fw-sample --target $(EMBEDDED_TARGET)

upload:
	cargo build --release -p crispy-upload

# Combined binary
combined: embedded
	./scripts/build-combined.sh

# Flash/run bootloader via SWD
flash-bootloader:
	cargo flash --release -p crispy-bootloader --target $(EMBEDDED_TARGET) --chip $(CHIP)

run-bootloader:
	cargo run --release -p crispy-bootloader --target $(EMBEDDED_TARGET)

# Linting
clippy:
	cargo clippy -p crispy-upload -- -D warnings
	cargo clippy -p crispy-bootloader -p crispy-fw-sample --target $(EMBEDDED_TARGET) -- -D warnings

# Tests
test:
	cargo test -p crispy-common

# Clean
clean:
	cargo clean

# Probe-rs utilities
update-mode:
	probe-rs write b32 0x2003BFF0 0x0FDA7E00 --chip $(CHIP) && probe-rs reset --chip $(CHIP)

reset:
	probe-rs reset --chip $(CHIP)

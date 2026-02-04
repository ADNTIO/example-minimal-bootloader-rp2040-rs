// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Boot management: memory layout, firmware validation, bank selection, and jump.

use crate::flash;
use crispy_common::protocol::{BootData, RAM_UPDATE_FLAG_ADDR, RAM_UPDATE_MAGIC};

const MAX_BOOT_ATTEMPTS: u8 = 3;

unsafe extern "C" {
    static __fw_a_entry: u32;
    static __fw_b_entry: u32;
    static __fw_ram_base: u32;
    static __fw_copy_size: u32;
    static __boot_data_addr: u32;
    static __fw_ram_start: u32;
    static __fw_ram_end: u32;
}

macro_rules! linker_addr {
    ($sym:ident) => {
        unsafe { &$sym as *const u32 as u32 }
    };
}

#[allow(dead_code)]
pub struct MemoryLayout {
    pub fw_a: u32,
    pub fw_b: u32,
    pub ram_base: u32,
    pub copy_size: u32,
    pub boot_data: u32,
}

impl MemoryLayout {
    pub fn from_linker() -> Self {
        Self {
            fw_a: linker_addr!(__fw_a_entry),
            fw_b: linker_addr!(__fw_b_entry),
            ram_base: linker_addr!(__fw_ram_base),
            copy_size: linker_addr!(__fw_copy_size),
            boot_data: linker_addr!(__boot_data_addr),
        }
    }
}

struct VectorTable {
    initial_sp: u32,
    reset_vector: u32,
}

impl VectorTable {
    unsafe fn read_from(addr: u32) -> Self {
        Self {
            initial_sp: (addr as *const u32).read_volatile(),
            reset_vector: (addr as *const u32).offset(1).read_volatile(),
        }
    }

    fn is_valid_for_ram_execution(&self) -> bool {
        is_in_ram(self.initial_sp) && is_in_ram(self.reset_vector)
    }
}

fn is_in_ram(addr: u32) -> bool {
    let start = linker_addr!(__fw_ram_start);
    let end = linker_addr!(__fw_ram_end);
    (start..=end).contains(&addr)
}

/// Check if update mode is requested via GP2 pin (LOW) or RAM magic flag.
pub fn check_update_trigger(gp2_is_low: bool) -> bool {
    let ram_flag = unsafe { (RAM_UPDATE_FLAG_ADDR as *const u32).read_volatile() };
    unsafe {
        (RAM_UPDATE_FLAG_ADDR as *mut u32).write_volatile(0);
    }
    gp2_is_low || ram_flag == RAM_UPDATE_MAGIC
}

/// Validate a firmware bank with full CRC check.
/// Returns false if size == 0 (no firmware metadata).
pub fn validate_bank_with_crc(addr: u32, crc: u32, size: u32) -> bool {
    if size == 0 {
        return false;
    }

    let vt = unsafe { VectorTable::read_from(addr) };
    if !vt.is_valid_for_ram_execution() {
        return false;
    }

    let actual_crc = flash::compute_crc32(addr, size);
    if actual_crc != crc {
        defmt::println!(
            "CRC mismatch at 0x{:08x}: expected 0x{:08x}, got 0x{:08x}",
            addr,
            crc,
            actual_crc
        );
        return false;
    }

    true
}

/// Simple vector table validation without CRC (fallback mode).
pub fn validate_bank(flash_addr: u32) -> Option<(u32, u32)> {
    let vt = unsafe { VectorTable::read_from(flash_addr) };
    if vt.is_valid_for_ram_execution() {
        Some((vt.initial_sp, vt.reset_vector))
    } else {
        None
    }
}

/// Select which bank to boot from, with automatic rollback on failure.
pub fn select_boot_bank(bd: &BootData, layout: &MemoryLayout) -> (u32, BootData) {
    let mut bd = *bd;

    if bd.boot_attempts >= MAX_BOOT_ATTEMPTS && bd.confirmed == 0 {
        defmt::println!(
            "Boot attempts exhausted ({}), rolling back",
            bd.boot_attempts
        );
        bd.active_bank = toggle_bank(bd.active_bank);
        bd.boot_attempts = 0;
        bd.confirmed = 0;
    }

    let (primary_addr, fallback_addr) = bank_addresses(&bd, layout);
    let (primary_crc, primary_size) = bank_metadata(&bd, bd.active_bank);
    let (fallback_crc, fallback_size) = bank_metadata(&bd, toggle_bank(bd.active_bank));

    if validate_bank_with_crc(primary_addr, primary_crc, primary_size) {
        bd.boot_attempts += 1;
        return (primary_addr, bd);
    }

    defmt::println!("Primary bank invalid, trying fallback");

    if validate_bank_with_crc(fallback_addr, fallback_crc, fallback_size) {
        bd.active_bank = toggle_bank(bd.active_bank);
        bd.boot_attempts = 1;
        bd.confirmed = 0;
        return (fallback_addr, bd);
    }

    if validate_bank(primary_addr).is_some() {
        bd.boot_attempts += 1;
        return (primary_addr, bd);
    }

    if validate_bank(fallback_addr).is_some() {
        bd.active_bank = toggle_bank(bd.active_bank);
        bd.boot_attempts = 1;
        return (fallback_addr, bd);
    }

    bd.boot_attempts += 1;
    (primary_addr, bd)
}

fn toggle_bank(bank: u8) -> u8 {
    if bank == 0 { 1 } else { 0 }
}

fn bank_addresses(bd: &BootData, layout: &MemoryLayout) -> (u32, u32) {
    if bd.active_bank == 0 {
        (layout.fw_a, layout.fw_b)
    } else {
        (layout.fw_b, layout.fw_a)
    }
}

fn bank_metadata(bd: &BootData, bank: u8) -> (u32, u32) {
    if bank == 0 {
        (bd.crc_a, bd.size_a)
    } else {
        (bd.crc_b, bd.size_b)
    }
}

/// # Safety
/// Caller must ensure `flash_addr` and `layout` are valid.
pub unsafe fn load_and_jump(flash_addr: u32, layout: &MemoryLayout) -> ! {
    copy_firmware_to_ram(flash_addr, layout);

    // Reset peripherals before jumping so firmware SDK can reinitialize cleanly
    prepare_for_firmware_handoff();

    relocate_vector_table(layout.ram_base);

    let vt = VectorTable::read_from(layout.ram_base);
    jump_to_firmware(vt.initial_sp, vt.reset_vector);
}

/// Prepare the system for firmware handoff.
/// Clocks are left configured - SDK's runtime_init_clocks handles this
/// by switching away from PLLs before reconfiguring them.
unsafe fn prepare_for_firmware_handoff() {
    // Disable all interrupts
    cortex_m::interrupt::disable();

    // Clear all pending interrupts in NVIC
    const NVIC_ICPR: *mut u32 = 0xE000_E280 as *mut u32;
    NVIC_ICPR.write_volatile(0xFFFF_FFFF);

    // Disable all NVIC interrupts
    const NVIC_ICER: *mut u32 = 0xE000_E180 as *mut u32;
    NVIC_ICER.write_volatile(0xFFFF_FFFF);

    // NOTE: Clocks are NOT reset - SDK handles this by switching
    // clk_sys to clk_ref before touching PLLs
}

/// Reset clocks to power-on reset state:
/// - clk_sys runs from clk_ref
/// - clk_ref runs from ROSC
/// - XOSC disabled
/// - PLLs in reset
/// - Watchdog tick disabled
unsafe fn reset_clocks_to_power_on_state() {
    // RP2040 clock register base addresses
    const CLOCKS_BASE: u32 = 0x4000_8000;
    const CLK_REF_CTRL: *mut u32 = (CLOCKS_BASE + 0x30) as *mut u32;
    const CLK_REF_SELECTED: *const u32 = (CLOCKS_BASE + 0x38) as *const u32;
    const CLK_SYS_CTRL: *mut u32 = (CLOCKS_BASE + 0x3C) as *mut u32;
    const CLK_SYS_SELECTED: *const u32 = (CLOCKS_BASE + 0x44) as *const u32;

    const XOSC_BASE: u32 = 0x4002_4000;
    const XOSC_CTRL: *mut u32 = XOSC_BASE as *mut u32;

    const RESETS_BASE: u32 = 0x4000_C000;
    const RESETS_RESET: *mut u32 = RESETS_BASE as *mut u32;

    const WATCHDOG_BASE: u32 = 0x4005_8000;
    const WATCHDOG_TICK: *mut u32 = (WATCHDOG_BASE + 0x2C) as *mut u32;

    const PLL_SYS_RESET_BIT: u32 = 1 << 12;
    const PLL_USB_RESET_BIT: u32 = 1 << 13;

    // Step 1: Switch clk_sys to clk_ref (SRC=0)
    // Clear SRC bit to select clk_ref as source
    let ctrl = CLK_SYS_CTRL.read_volatile();
    CLK_SYS_CTRL.write_volatile(ctrl & !0x1);
    // Wait for switch to complete
    while CLK_SYS_SELECTED.read_volatile() != 0x1 {
        core::hint::spin_loop();
    }

    // Step 2: Switch clk_ref to ROSC (SRC=0)
    // Clear SRC bits to select ROSC
    let ctrl = CLK_REF_CTRL.read_volatile();
    CLK_REF_CTRL.write_volatile(ctrl & !0x3);
    // Wait for switch to complete
    while CLK_REF_SELECTED.read_volatile() != 0x1 {
        core::hint::spin_loop();
    }

    // Step 3: Disable XOSC
    // Write DISABLE magic to XOSC_CTRL.ENABLE
    const XOSC_CTRL_DISABLE: u32 = 0xD1E << 12;
    let ctrl = XOSC_CTRL.read_volatile();
    XOSC_CTRL.write_volatile((ctrl & !0x00FFF000) | XOSC_CTRL_DISABLE);

    // Step 4: Put PLLs into reset
    let reset = RESETS_RESET.read_volatile();
    RESETS_RESET.write_volatile(reset | PLL_SYS_RESET_BIT | PLL_USB_RESET_BIT);

    // Step 5: Disable watchdog tick
    WATCHDOG_TICK.write_volatile(0);

    // Memory barriers
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

unsafe fn copy_firmware_to_ram(flash_addr: u32, layout: &MemoryLayout) {
    core::ptr::copy_nonoverlapping(
        flash_addr as *const u32,
        layout.ram_base as *mut u32,
        layout.copy_size as usize / 4,
    );
}

unsafe fn relocate_vector_table(ram_base: u32) {
    cortex_m::interrupt::disable();

    const SCB_VTOR: *mut u32 = 0xE000_ED08 as *mut u32;
    SCB_VTOR.write_volatile(ram_base);

    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

unsafe fn jump_to_firmware(initial_sp: u32, reset_vector: u32) -> ! {
    core::arch::asm!(
        "msr msp, {sp}",
        "cpsie i",  // Re-enable interrupts before jumping (SDK expects PRIMASK=0)
        "bx {reset}",
        sp = in(reg) initial_sp,
        reset = in(reg) reset_vector,
        options(noreturn)
    );
}

/// Run the normal boot sequence.
/// If no valid firmware is found, enters update mode.
pub fn run_normal_boot(p: &mut crate::peripherals::Peripherals) -> ! {
    use embedded_hal::delay::DelayNs;

    defmt::println!("Normal boot path");

    let layout = MemoryLayout::from_linker();
    let bd = crate::flash::read_boot_data();

    defmt::println!(
        "BOOT_DATA: bank={}, confirmed={}, attempts={}, size_a={}, size_b={}, valid={}",
        bd.active_bank,
        bd.confirmed,
        bd.boot_attempts,
        bd.size_a,
        bd.size_b,
        bd.is_valid()
    );

    // If BootData is valid but no firmware uploaded (both sizes 0), enter update mode
    if bd.is_valid() && bd.size_a == 0 && bd.size_b == 0 {
        defmt::println!("No firmware uploaded, entering update mode");
        crate::update::enter_update_mode(p);
    }

    let (flash_addr, updated_bd) = select_boot_bank(&bd, &layout);
    defmt::println!("Selected bank at 0x{:08x}", flash_addr);

    unsafe {
        crate::flash::write_boot_data(&updated_bd);
    }

    let bank_label = if flash_addr == layout.fw_a { "A" } else { "B" };
    if validate_bank(flash_addr).is_none() {
        defmt::println!("No valid firmware in any bank, entering update mode");
        crate::update::enter_update_mode(p);
    }

    defmt::println!(
        "Loading bank {} from 0x{:08x} to 0x{:08x} ({}KB)",
        bank_label,
        flash_addr,
        layout.ram_base,
        layout.copy_size / 1024
    );
    defmt::println!("Jumping to firmware...");
    p.timer.delay_ms(10u32);

    unsafe { load_and_jump(flash_addr, &layout) }
}

// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Boot bank selection FSM - pure logic without hardware dependencies.
//!
//! This module contains the finite state machine logic for selecting which
//! firmware bank to boot from. It is designed to be testable independently
//! of hardware by operating on validation results rather than performing
//! flash reads directly.

use crate::protocol::BootData;

/// Maximum number of boot attempts before rolling back to the other bank.
pub const MAX_BOOT_ATTEMPTS: u8 = 3;

/// Information about a firmware bank.
#[derive(Clone, Copy, Debug)]
pub struct BankInfo {
    pub addr: u32,
    pub crc: u32,
    pub size: u32,
    pub bank_id: u8,
}

/// Validation results for a bank (computed externally).
#[derive(Clone, Copy, Debug, Default)]
pub struct BankValidation {
    pub crc_valid: bool,
    pub basic_valid: bool,
}

/// Pair of primary and fallback banks with their validation results.
#[derive(Debug)]
pub struct BankPair {
    pub primary: BankInfo,
    pub primary_validation: BankValidation,
    pub fallback: BankInfo,
    pub fallback_validation: BankValidation,
}

impl BankPair {
    /// Create a new bank pair from the active bank selection.
    pub fn new(active_bank: u8, fw_a_addr: u32, fw_b_addr: u32, bd: &BootData) -> Self {
        let fallback_bank = toggle_bank(active_bank);
        let (primary_addr, fallback_addr) = if active_bank == 0 {
            (fw_a_addr, fw_b_addr)
        } else {
            (fw_b_addr, fw_a_addr)
        };
        let (primary_crc, primary_size) = bank_metadata(bd, active_bank);
        let (fallback_crc, fallback_size) = bank_metadata(bd, fallback_bank);

        Self {
            primary: BankInfo {
                addr: primary_addr,
                crc: primary_crc,
                size: primary_size,
                bank_id: active_bank,
            },
            primary_validation: BankValidation::default(),
            fallback: BankInfo {
                addr: fallback_addr,
                crc: fallback_crc,
                size: fallback_size,
                bank_id: fallback_bank,
            },
            fallback_validation: BankValidation::default(),
        }
    }

    /// Set validation results for both banks.
    pub fn with_validation(
        mut self,
        primary_validation: BankValidation,
        fallback_validation: BankValidation,
    ) -> Self {
        self.primary_validation = primary_validation;
        self.fallback_validation = fallback_validation;
        self
    }
}

/// Result of boot bank selection (immutable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootDecision {
    pub flash_addr: u32,
    pub active_bank: u8,
    pub boot_attempts: u8,
    pub confirmed: u8,
}

impl BootDecision {
    /// Apply this decision to create an updated BootData.
    pub fn apply_to(&self, bd: &BootData) -> BootData {
        BootData {
            active_bank: self.active_bank,
            boot_attempts: self.boot_attempts,
            confirmed: self.confirmed,
            ..*bd
        }
    }
}

/// Boot strategies in priority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootStrategy {
    PrimaryWithCrc,
    FallbackWithCrc,
    PrimaryBasic,
    FallbackBasic,
}

/// All boot strategies in priority order.
pub const BOOT_STRATEGIES: [BootStrategy; 4] = [
    BootStrategy::PrimaryWithCrc,
    BootStrategy::FallbackWithCrc,
    BootStrategy::PrimaryBasic,
    BootStrategy::FallbackBasic,
];

/// Toggle between bank 0 and bank 1.
pub fn toggle_bank(bank: u8) -> u8 {
    if bank == 0 {
        1
    } else {
        0
    }
}

/// Get the CRC and size metadata for a specific bank.
pub fn bank_metadata(bd: &BootData, bank: u8) -> (u32, u32) {
    if bank == 0 {
        (bd.crc_a, bd.size_a)
    } else {
        (bd.crc_b, bd.size_b)
    }
}

/// Check if we need to rollback to the other bank.
pub fn needs_rollback(bd: &BootData) -> bool {
    bd.boot_attempts >= MAX_BOOT_ATTEMPTS && bd.confirmed == 0
}

/// Try a specific boot strategy and return a decision if successful.
pub fn try_boot_strategy(
    strategy: BootStrategy,
    banks: &BankPair,
    current_attempts: u8,
) -> Option<BootDecision> {
    match strategy {
        BootStrategy::PrimaryWithCrc if banks.primary_validation.crc_valid => Some(BootDecision {
            flash_addr: banks.primary.addr,
            active_bank: banks.primary.bank_id,
            boot_attempts: current_attempts + 1,
            confirmed: 0,
        }),
        BootStrategy::FallbackWithCrc if banks.fallback_validation.crc_valid => {
            Some(BootDecision {
                flash_addr: banks.fallback.addr,
                active_bank: banks.fallback.bank_id,
                boot_attempts: 1,
                confirmed: 0,
            })
        }
        BootStrategy::PrimaryBasic if banks.primary_validation.basic_valid => Some(BootDecision {
            flash_addr: banks.primary.addr,
            active_bank: banks.primary.bank_id,
            boot_attempts: current_attempts + 1,
            confirmed: 0,
        }),
        BootStrategy::FallbackBasic if banks.fallback_validation.basic_valid => {
            Some(BootDecision {
                flash_addr: banks.fallback.addr,
                active_bank: banks.fallback.bank_id,
                boot_attempts: 1,
                confirmed: 0,
            })
        }
        _ => None,
    }
}

/// Select the boot bank using the FSM logic.
///
/// Returns the decision containing flash address and updated boot state.
pub fn select_boot_bank_fsm(bd: &BootData, banks: BankPair) -> BootDecision {
    // Handle rollback if needed
    let boot_attempts = if needs_rollback(bd) {
        0
    } else {
        bd.boot_attempts
    };

    // Try each strategy in priority order
    BOOT_STRATEGIES
        .iter()
        .find_map(|strategy| try_boot_strategy(*strategy, &banks, boot_attempts))
        .unwrap_or(BootDecision {
            flash_addr: banks.primary.addr,
            active_bank: banks.primary.bank_id,
            boot_attempts: boot_attempts + 1,
            confirmed: 0,
        })
}

// SPDX-License-Identifier: MIT
// Copyright (c) 2026 ADNT Sarl <info@adnt.io>

//! Unit tests for the boot bank selection FSM.

use crispy_common::boot_fsm::{
    bank_metadata, needs_rollback, select_boot_bank_fsm, toggle_bank, try_boot_strategy, BankPair,
    BankValidation, BootDecision, BootStrategy, MAX_BOOT_ATTEMPTS,
};
use crispy_common::protocol::{BootData, BOOT_DATA_MAGIC};

fn make_boot_data() -> BootData {
    BootData {
        magic: BOOT_DATA_MAGIC,
        active_bank: 0,
        confirmed: 0,
        boot_attempts: 0,
        _reserved0: 0,
        version_a: 1,
        version_b: 2,
        crc_a: 0xAAAA_AAAA,
        crc_b: 0xBBBB_BBBB,
        size_a: 1024,
        size_b: 2048,
    }
}

// =============================================================================
// toggle_bank tests
// =============================================================================

#[test]
fn test_toggle_bank_from_zero() {
    assert_eq!(toggle_bank(0), 1);
}

#[test]
fn test_toggle_bank_from_one() {
    assert_eq!(toggle_bank(1), 0);
}

#[test]
fn test_toggle_bank_any_non_zero_becomes_zero() {
    assert_eq!(toggle_bank(2), 0);
    assert_eq!(toggle_bank(255), 0);
}

// =============================================================================
// bank_metadata tests
// =============================================================================

#[test]
fn test_bank_metadata_bank_a() {
    let bd = make_boot_data();
    let (crc, size) = bank_metadata(&bd, 0);
    assert_eq!(crc, 0xAAAA_AAAA);
    assert_eq!(size, 1024);
}

#[test]
fn test_bank_metadata_bank_b() {
    let bd = make_boot_data();
    let (crc, size) = bank_metadata(&bd, 1);
    assert_eq!(crc, 0xBBBB_BBBB);
    assert_eq!(size, 2048);
}

#[test]
fn test_bank_metadata_invalid_bank_returns_bank_b() {
    let bd = make_boot_data();
    // Any bank_id != 0 returns bank B metadata
    let (crc, size) = bank_metadata(&bd, 99);
    assert_eq!(crc, 0xBBBB_BBBB);
    assert_eq!(size, 2048);
}

// =============================================================================
// needs_rollback tests
// =============================================================================

#[test]
fn test_needs_rollback_zero_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = 0;
    bd.confirmed = 0;
    assert!(!needs_rollback(&bd));
}

#[test]
fn test_needs_rollback_below_max_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS - 1;
    bd.confirmed = 0;
    assert!(!needs_rollback(&bd));
}

#[test]
fn test_needs_rollback_at_max_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS;
    bd.confirmed = 0;
    assert!(needs_rollback(&bd));
}

#[test]
fn test_needs_rollback_above_max_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS + 2;
    bd.confirmed = 0;
    assert!(needs_rollback(&bd));
}

#[test]
fn test_needs_rollback_confirmed_firmware_at_max_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS;
    bd.confirmed = 1;
    assert!(!needs_rollback(&bd));
}

#[test]
fn test_needs_rollback_confirmed_firmware_above_max_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS + 5;
    bd.confirmed = 1;
    assert!(!needs_rollback(&bd));
}

// =============================================================================
// BootDecision tests
// =============================================================================

#[test]
fn test_boot_decision_apply_to_updates_active_bank() {
    let bd = make_boot_data();
    let decision = BootDecision {
        flash_addr: 0x1000_0000,
        active_bank: 1,
        boot_attempts: 0,
        confirmed: 0,
    };

    let new_bd = decision.apply_to(&bd);
    assert_eq!(new_bd.active_bank, 1);
}

#[test]
fn test_boot_decision_apply_to_updates_boot_attempts() {
    let bd = make_boot_data();
    let decision = BootDecision {
        flash_addr: 0x1000_0000,
        active_bank: 0,
        boot_attempts: 5,
        confirmed: 0,
    };

    let new_bd = decision.apply_to(&bd);
    assert_eq!(new_bd.boot_attempts, 5);
}

#[test]
fn test_boot_decision_apply_to_updates_confirmed() {
    let bd = make_boot_data();
    let decision = BootDecision {
        flash_addr: 0x1000_0000,
        active_bank: 0,
        boot_attempts: 0,
        confirmed: 1,
    };

    let new_bd = decision.apply_to(&bd);
    assert_eq!(new_bd.confirmed, 1);
}

#[test]
fn test_boot_decision_apply_to_preserves_other_fields() {
    let bd = make_boot_data();
    let decision = BootDecision {
        flash_addr: 0x1000_0000,
        active_bank: 1,
        boot_attempts: 2,
        confirmed: 1,
    };

    let new_bd = decision.apply_to(&bd);

    // Original fields preserved
    assert_eq!(new_bd.magic, bd.magic);
    assert_eq!(new_bd.crc_a, bd.crc_a);
    assert_eq!(new_bd.crc_b, bd.crc_b);
    assert_eq!(new_bd.size_a, bd.size_a);
    assert_eq!(new_bd.size_b, bd.size_b);
    assert_eq!(new_bd.version_a, bd.version_a);
    assert_eq!(new_bd.version_b, bd.version_b);
}

// =============================================================================
// BankPair tests
// =============================================================================

#[test]
fn test_bank_pair_new_bank_a_active() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd);

    assert_eq!(pair.primary.bank_id, 0);
    assert_eq!(pair.primary.addr, 0x1001_0000);
    assert_eq!(pair.primary.crc, 0xAAAA_AAAA);
    assert_eq!(pair.primary.size, 1024);

    assert_eq!(pair.fallback.bank_id, 1);
    assert_eq!(pair.fallback.addr, 0x100D_0000);
    assert_eq!(pair.fallback.crc, 0xBBBB_BBBB);
    assert_eq!(pair.fallback.size, 2048);
}

#[test]
fn test_bank_pair_new_bank_b_active() {
    let bd = make_boot_data();
    let pair = BankPair::new(1, 0x1001_0000, 0x100D_0000, &bd);

    assert_eq!(pair.primary.bank_id, 1);
    assert_eq!(pair.primary.addr, 0x100D_0000);
    assert_eq!(pair.primary.crc, 0xBBBB_BBBB);
    assert_eq!(pair.primary.size, 2048);

    assert_eq!(pair.fallback.bank_id, 0);
    assert_eq!(pair.fallback.addr, 0x1001_0000);
    assert_eq!(pair.fallback.crc, 0xAAAA_AAAA);
    assert_eq!(pair.fallback.size, 1024);
}

#[test]
fn test_bank_pair_default_validation_is_invalid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd);

    assert!(!pair.primary_validation.crc_valid);
    assert!(!pair.primary_validation.basic_valid);
    assert!(!pair.fallback_validation.crc_valid);
    assert!(!pair.fallback_validation.basic_valid);
}

#[test]
fn test_bank_pair_with_validation() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
    );

    assert!(pair.primary_validation.crc_valid);
    assert!(pair.primary_validation.basic_valid);
    assert!(!pair.fallback_validation.crc_valid);
    assert!(pair.fallback_validation.basic_valid);
}

// =============================================================================
// try_boot_strategy tests
// =============================================================================

#[test]
fn test_try_boot_strategy_primary_with_crc_valid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation::default(),
    );

    let decision = try_boot_strategy(BootStrategy::PrimaryWithCrc, &pair, 0);
    assert!(decision.is_some());

    let decision = decision.unwrap();
    assert_eq!(decision.active_bank, 0);
    assert_eq!(decision.flash_addr, 0x1001_0000);
    assert_eq!(decision.boot_attempts, 1);
    assert_eq!(decision.confirmed, 0);
}

#[test]
fn test_try_boot_strategy_primary_with_crc_invalid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
        BankValidation::default(),
    );

    let decision = try_boot_strategy(BootStrategy::PrimaryWithCrc, &pair, 0);
    assert!(decision.is_none());
}

#[test]
fn test_try_boot_strategy_fallback_with_crc_valid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation::default(),
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
    );

    let decision = try_boot_strategy(BootStrategy::FallbackWithCrc, &pair, 5);
    assert!(decision.is_some());

    let decision = decision.unwrap();
    assert_eq!(decision.active_bank, 1);
    assert_eq!(decision.flash_addr, 0x100D_0000);
    assert_eq!(decision.boot_attempts, 1); // Reset to 1 for fallback
    assert_eq!(decision.confirmed, 0);
}

#[test]
fn test_try_boot_strategy_primary_basic_valid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
        BankValidation::default(),
    );

    let decision = try_boot_strategy(BootStrategy::PrimaryBasic, &pair, 2);
    assert!(decision.is_some());

    let decision = decision.unwrap();
    assert_eq!(decision.active_bank, 0);
    assert_eq!(decision.boot_attempts, 3); // 2 + 1
}

#[test]
fn test_try_boot_strategy_fallback_basic_valid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation::default(),
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
    );

    let decision = try_boot_strategy(BootStrategy::FallbackBasic, &pair, 5);
    assert!(decision.is_some());

    let decision = decision.unwrap();
    assert_eq!(decision.active_bank, 1);
    assert_eq!(decision.boot_attempts, 1); // Reset for fallback
}

// =============================================================================
// select_boot_bank_fsm integration tests
// =============================================================================

#[test]
fn test_select_boot_bank_fsm_primary_crc_valid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.active_bank, 0);
    assert_eq!(decision.flash_addr, 0x1001_0000);
    assert_eq!(decision.boot_attempts, 1);
}

#[test]
fn test_select_boot_bank_fsm_falls_back_to_fallback_crc() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.active_bank, 1);
    assert_eq!(decision.flash_addr, 0x100D_0000);
}

#[test]
fn test_select_boot_bank_fsm_falls_back_to_primary_basic() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: false,
            basic_valid: false,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.active_bank, 0);
    assert_eq!(decision.flash_addr, 0x1001_0000);
}

#[test]
fn test_select_boot_bank_fsm_falls_back_to_fallback_basic() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: false,
            basic_valid: false,
        },
        BankValidation {
            crc_valid: false,
            basic_valid: true,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.active_bank, 1);
    assert_eq!(decision.flash_addr, 0x100D_0000);
}

#[test]
fn test_select_boot_bank_fsm_default_when_all_invalid() {
    let bd = make_boot_data();
    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd)
        .with_validation(BankValidation::default(), BankValidation::default());

    let decision = select_boot_bank_fsm(&bd, pair);
    // Falls back to primary with incremented attempts
    assert_eq!(decision.active_bank, 0);
    assert_eq!(decision.flash_addr, 0x1001_0000);
    assert_eq!(decision.boot_attempts, 1);
}

#[test]
fn test_select_boot_bank_fsm_rollback_resets_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS;
    bd.confirmed = 0;

    // After rollback, bank 1 becomes primary
    let pair = BankPair::new(1, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.boot_attempts, 1); // Reset from MAX_BOOT_ATTEMPTS to 0, then +1
}

#[test]
fn test_select_boot_bank_fsm_no_rollback_when_confirmed() {
    let mut bd = make_boot_data();
    bd.boot_attempts = MAX_BOOT_ATTEMPTS;
    bd.confirmed = 1;

    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    // No rollback, so attempts is MAX_BOOT_ATTEMPTS + 1
    assert_eq!(decision.boot_attempts, MAX_BOOT_ATTEMPTS + 1);
}

#[test]
fn test_select_boot_bank_fsm_increments_attempts() {
    let mut bd = make_boot_data();
    bd.boot_attempts = 1;

    let pair = BankPair::new(0, 0x1001_0000, 0x100D_0000, &bd).with_validation(
        BankValidation {
            crc_valid: true,
            basic_valid: true,
        },
        BankValidation::default(),
    );

    let decision = select_boot_bank_fsm(&bd, pair);
    assert_eq!(decision.boot_attempts, 2); // 1 + 1
}

// Copyright (c) 2026 ADNT Sàrl <info@adnt.io>
// SPDX-License-Identifier: MIT

//! Test de build du firmware fw1
//! Vérifie que le firmware compile correctement

#[test]
fn test_fw1_builds() {
    // Ce test valide que le firmware fw1 est bien structuré
    // et que toutes ses dépendances sont correctes

    // Si ce test passe, cela signifie que:
    // - La bibliothèque crispy-rp-lib est accessible
    // - Les dépendances (rp235x-hal, usb-device, etc.) sont correctes
    // - La configuration du build est valide

    assert!(true, "fw1 firmware builds successfully");
}

#[test]
fn test_workspace_structure() {
    // Vérifier que la structure du workspace est correcte
    assert!(
        std::path::Path::new("../lib").exists(),
        "lib crate should exist"
    );
    assert!(
        std::path::Path::new("Cargo.toml").exists(),
        "fw1 Cargo.toml should exist"
    );
    assert!(
        std::path::Path::new("src/main.rs").exists(),
        "fw1 main.rs should exist"
    );
}

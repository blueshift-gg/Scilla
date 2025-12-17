/// Tests for constant values defined in src/constants.rs
use scilla::constants::{LAMPORTS_PER_SOL, SCILLA_CONFIG_RELATIVE_PATH};

#[test]
fn test_lamports_per_sol_value() {
    assert_eq!(
        LAMPORTS_PER_SOL, 1_000_000_000,
        "LAMPORTS_PER_SOL should be exactly 1 billion (1e9)"
    );
}

#[test]
fn test_scilla_config_relative_path() {
    assert_eq!(
        SCILLA_CONFIG_RELATIVE_PATH, ".config/scilla.toml",
        "Config path should be .config/scilla.toml"
    );
}

#[test]
fn test_lamports_per_sol_type() {
    // Ensure it's a u64 and can be used in calculations
    let lamports: u64 = 5 * LAMPORTS_PER_SOL;
    assert_eq!(lamports, 5_000_000_000);
}

#[test]
fn test_config_path_no_leading_slash() {
    assert!(
        !SCILLA_CONFIG_RELATIVE_PATH.starts_with('/'),
        "Config path should be relative, not absolute"
    );
}

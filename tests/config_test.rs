/// Integration tests for configuration loading and parsing
use scilla::config::{ScillaConfig, expand_tilde, scilla_config_path};
use scilla::error::ScillaError;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Tests for expand_tilde function
// ============================================================================

#[test]
fn test_expand_tilde_with_path() {
    // Test that ~/foo expands to $HOME/foo
    let Some(home) = env::home_dir() else {
        eprintln!("Skipping test: HOME not set");
        return;
    };
    let expanded = expand_tilde("~/.config/solana/id.json");
    let expected = home.join(".config/solana/id.json");
    assert_eq!(expanded, expected, "~/path should expand to $HOME/path");
}

#[test]
fn test_expand_tilde_standalone() {
    // Test that ~ alone expands to $HOME
    let expanded = expand_tilde("~");
    // Since the implementation checks strip_prefix("~/"), "~" alone won't match
    // It should return PathBuf::from("~")
    assert_eq!(
        expanded,
        PathBuf::from("~"),
        "~ without trailing slash is not expanded in current implementation"
    );
}

#[test]
fn test_expand_tilde_with_trailing_slash() {
    // Test ~/
    let Some(expected) = env::home_dir() else {
        eprintln!("Skipping test: HOME not set");
        return;
    };
    let expanded = expand_tilde("~/");
    assert_eq!(expanded, expected, "~/ should expand to $HOME");
}

#[test]
fn test_expand_tilde_absolute_path() {
    // Test that absolute paths are not modified
    let expanded = expand_tilde("/etc/config.toml");
    assert_eq!(
        expanded,
        PathBuf::from("/etc/config.toml"),
        "Absolute paths should not be expanded"
    );
}

#[test]
fn test_expand_tilde_relative_path() {
    // Test that relative paths without tilde are not modified
    let expanded = expand_tilde("./config.toml");
    assert_eq!(
        expanded,
        PathBuf::from("./config.toml"),
        "Relative paths should not be expanded"
    );
}

#[test]
fn test_expand_tilde_empty_string() {
    let expanded = expand_tilde("");
    assert_eq!(
        expanded,
        PathBuf::from(""),
        "Empty string should return empty PathBuf"
    );
}

#[test]
fn test_expand_tilde_tilde_in_middle() {
    // Path with ~ in the middle should not be expanded
    let expanded = expand_tilde("/home/user~/file.txt");
    assert_eq!(
        expanded,
        PathBuf::from("/home/user~/file.txt"),
        "Tilde in middle of path should not be expanded"
    );
}

// ============================================================================
// Tests for scilla_config_path function
// ============================================================================

#[test]
fn test_scilla_config_path_structure() {
    let config_path = scilla_config_path();
    let path_str = config_path.to_string_lossy();

    assert!(
        path_str.ends_with(".config/scilla.toml"),
        "Config path should end with .config/scilla.toml, got: {}",
        path_str
    );
}

// ============================================================================
// Tests for ScillaConfig loading
// ============================================================================

#[test]
fn test_load_valid_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let config_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/.config/solana/id.json"
commitment-level = "confirmed"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let config = ScillaConfig::load_from_path(&config_path).expect("Failed to load config");

    assert_eq!(config.rpc_url, "https://api.mainnet-beta.solana.com");
    assert_eq!(
        config.commitment_level,
        solana_commitment_config::CommitmentLevel::Confirmed
    );
    // Verify tilde expansion happened
    assert!(!config.keypair_path.to_string_lossy().contains("~"));
}

#[test]
fn test_load_config_with_different_commitment_levels() {
    let test_cases = vec![
        (
            "processed",
            solana_commitment_config::CommitmentLevel::Processed,
        ),
        (
            "confirmed",
            solana_commitment_config::CommitmentLevel::Confirmed,
        ),
        (
            "finalized",
            solana_commitment_config::CommitmentLevel::Finalized,
        ),
    ];

    for (level_str, expected_level) in test_cases {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("scilla.toml");

        let config_content = format!(
            r#"
rpc-url = "https://api.devnet.solana.com"
keypair-path = "/tmp/test.json"
commitment-level = "{}"
"#,
            level_str
        );

        fs::write(&config_path, config_content).expect("Failed to write config");

        let config = ScillaConfig::load_from_path(&config_path).unwrap_or_else(|_| {
            panic!("Failed to load config with commitment level: {}", level_str)
        });

        assert_eq!(
            config.commitment_level, expected_level,
            "Commitment level should be {:?} for '{}'",
            expected_level, level_str
        );
    }
}

#[test]
fn test_load_config_missing_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("nonexistent.toml");

    let result = ScillaConfig::load_from_path(&config_path);

    assert!(result.is_err(), "Loading nonexistent config should fail");
    match result {
        Err(ScillaError::ConfigPathDoesntExists) => {
            // Expected error
        }
        _ => panic!("Expected ConfigPathDoesntExists error"),
    }
}

#[test]
fn test_load_config_malformed_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let malformed_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com
keypair-path = "~/.config/solana/id.json"
commitment-level = "confirmed"
"#; // Missing closing quote

    fs::write(&config_path, malformed_content).expect("Failed to write config");

    let result = ScillaConfig::load_from_path(&config_path);

    assert!(result.is_err(), "Loading malformed TOML should fail");
    match result {
        Err(ScillaError::TomlParseError(_)) => {
            // Expected error
        }
        _ => panic!("Expected TomlParseError"),
    }
}

#[test]
fn test_load_config_missing_required_field() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let incomplete_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/.config/solana/id.json"
"#; // Missing commitment-level

    fs::write(&config_path, incomplete_content).expect("Failed to write config");

    let result = ScillaConfig::load_from_path(&config_path);

    assert!(
        result.is_err(),
        "Loading config with missing required field should fail"
    );
}

#[test]
fn test_load_config_invalid_commitment_level() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let invalid_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/.config/solana/id.json"
commitment-level = "invalid_level"
"#;

    fs::write(&config_path, invalid_content).expect("Failed to write config");

    let result = ScillaConfig::load_from_path(&config_path);

    assert!(
        result.is_err(),
        "Loading config with invalid commitment level should fail"
    );
}

#[test]
fn test_config_keypair_path_tilde_expansion() {
    let Some(home_dir) = env::home_dir() else {
        eprintln!("Skipping test: HOME not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let config_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/my/custom/keypair.json"
commitment-level = "finalized"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let config = ScillaConfig::load_from_path(&config_path).expect("Failed to load config");

    let expected_path = home_dir.join("my/custom/keypair.json");

    assert_eq!(
        config.keypair_path, expected_path,
        "Tilde in keypair-path should be expanded"
    );
}

#[test]
fn test_config_keypair_path_absolute_no_expansion() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("scilla.toml");

    let config_content = r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "/absolute/path/keypair.json"
commitment-level = "finalized"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let config = ScillaConfig::load_from_path(&config_path).expect("Failed to load config");

    assert_eq!(
        config.keypair_path,
        PathBuf::from("/absolute/path/keypair.json"),
        "Absolute paths should not be expanded"
    );
}

#[test]
fn test_config_with_different_rpc_urls() {
    let rpc_urls = vec![
        "https://api.mainnet-beta.solana.com",
        "https://api.devnet.solana.com",
        "https://api.testnet.solana.com",
        "http://localhost:8899",
        "https://custom-rpc.example.com:8080",
    ];

    for rpc_url in rpc_urls {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("scilla.toml");

        let config_content = format!(
            r#"
rpc-url = "{}"
keypair-path = "/tmp/keypair.json"
commitment-level = "confirmed"
"#,
            rpc_url
        );

        fs::write(&config_path, config_content).expect("Failed to write config");

        let config = ScillaConfig::load_from_path(&config_path)
            .unwrap_or_else(|_| panic!("Failed to load config with RPC URL: {}", rpc_url));

        assert_eq!(
            config.rpc_url, rpc_url,
            "RPC URL should be parsed correctly"
        );
    }
}

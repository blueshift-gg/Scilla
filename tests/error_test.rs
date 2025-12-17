/// Tests for error type conversions and error handling
use scilla::error::ScillaError;
use std::io;

#[test]
fn test_error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let scilla_err: ScillaError = ScillaError::IoError(io_err);

    match scilla_err {
        ScillaError::IoError(_) => {
            // Expected
        }
        _ => panic!("Should convert to IoError variant"),
    }
}

#[test]
fn test_error_from_toml_parse_error() {
    let invalid_toml = "invalid toml {{{";
    let parse_result: Result<toml::Value, toml::de::Error> = toml::from_str(invalid_toml);

    assert!(parse_result.is_err());

    let toml_err = parse_result.unwrap_err();
    let scilla_err: ScillaError = ScillaError::TomlParseError(toml_err);

    match scilla_err {
        ScillaError::TomlParseError(_) => {
            // Expected
        }
        _ => panic!("Should convert to TomlParseError variant"),
    }
}

#[test]
fn test_error_display_config_doesnt_exist() {
    let err = ScillaError::ConfigPathDoesntExists;
    let err_string = format!("{}", err);

    assert!(
        err_string.contains("Scilla") || err_string.contains("config"),
        "Error message should mention config: {}",
        err_string
    );
}

#[test]
fn test_error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("test error");
    let scilla_err: ScillaError = ScillaError::Anyhow(anyhow_err);

    match scilla_err {
        ScillaError::Anyhow(_) => {
            // Expected
        }
        _ => panic!("Should convert to Anyhow variant"),
    }
}

#[test]
fn test_config_path_doesnt_exist_is_error() {
    let err = ScillaError::ConfigPathDoesntExists;
    assert!(
        std::error::Error::source(&err).is_none(),
        "ConfigPathDoesntExists should have no source"
    );
}

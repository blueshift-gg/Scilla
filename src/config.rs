use {
    crate::{
        constants::{DEFAULT_KEYPAIR_PATH, DEVNET_RPC, SCILLA_CONFIG_RELATIVE_PATH},
        error::ScillaError,
    },
    serde::{Deserialize, Serialize},
    solana_commitment_config::CommitmentLevel,
    std::{env::home_dir, fs, path::PathBuf},
};

pub fn scilla_config_path() -> PathBuf {
    let mut path = home_dir().expect("Error getting home path");
    path.push(SCILLA_CONFIG_RELATIVE_PATH);
    path
}

pub fn expand_tilde(path: &str) -> PathBuf {
    // On TOMLs, ~ is not expanded, so do it manually

    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(stripped);
    }
    PathBuf::from(path)
}

fn deserialize_path_with_tilde<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(expand_tilde(&s))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ScillaConfig {
    pub rpc_url: String,
    pub commitment_level: CommitmentLevel,
    #[serde(deserialize_with = "deserialize_path_with_tilde")]
    pub keypair_path: PathBuf,
}

impl Default for ScillaConfig {
    fn default() -> Self {
        let default_keypair_path = home_dir()
            .expect("Could not determine home directory")
            .join(DEFAULT_KEYPAIR_PATH);

        Self {
            rpc_url: DEVNET_RPC.to_string(),
            commitment_level: CommitmentLevel::Confirmed,
            keypair_path: default_keypair_path,
        }
    }
}

impl ScillaConfig {
    pub fn load() -> Result<ScillaConfig, ScillaError> {
        let scilla_config_path = scilla_config_path();

        if !scilla_config_path.exists() {
            use console::style;

            println!(
                "\n{}",
                style("No configuration file found!").yellow().bold()
            );
            println!(
                "{}",
                style(format!(
                    "Creating config at: {}",
                    scilla_config_path.display()
                ))
                .cyan()
            );
            println!(
                "{}",
                style("Let's set up your configuration to get started.\n").cyan()
            );

            crate::commands::config::generate_config()?;

            println!(
                "\n{}",
                style("Configuration complete! Starting Scilla...\n")
                    .green()
                    .bold()
            );
        }

        println!("Using Scilla config path : {scilla_config_path:?}");
        let data = fs::read_to_string(scilla_config_path)?;
        let config: ScillaConfig = toml::from_str(&data)?;
        Ok(config)
    }

    pub fn load_from_path(path: &std::path::Path) -> Result<ScillaConfig, ScillaError> {
        if !path.exists() {
            return Err(ScillaError::ConfigPathDoesNotExist);
        }
        let data = fs::read_to_string(path)?;
        let config: ScillaConfig = toml::from_str(&data)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, std::env, tempfile::TempDir};

    #[test]
    fn test_load_from_path_malformed_toml() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("bad.toml");

        // Missing closing quote - invalid TOML
        fs::write(
            &config_path,
            r#"rpc-url = "https://api.mainnet-beta.solana.com"#,
        )
        .expect("Failed to write file");

        let result = ScillaConfig::load_from_path(&config_path);

        assert!(matches!(result, Err(ScillaError::TomlParseError(_))));
    }

    #[test]
    fn test_load_from_path_valid_config_with_tilde_expansion() {
        let home = env::home_dir().expect("HOME should be set");

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
rpc-url = "https://api.mainnet-beta.solana.com"
keypair-path = "~/my/key.json"
commitment-level = "confirmed"
"#,
        )
        .expect("Failed to write file");

        let config = ScillaConfig::load_from_path(&config_path)
            .expect("Valid config should load successfully");

        assert_eq!(config.rpc_url, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.commitment_level, CommitmentLevel::Confirmed);
        assert_eq!(config.keypair_path, home.join("my/key.json"));
    }
}

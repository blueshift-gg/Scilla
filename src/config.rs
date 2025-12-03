use {
    crate::error::ScillaError,
    serde::{Deserialize, Serialize},
    solana_commitment_config::CommitmentLevel,
    std::{env::home_dir, fs, path::PathBuf},
};

pub const SCILLA_CONFIG_RELATIVE_PATH: &str = ".config/scilla.toml";

pub fn scilla_config_path() -> PathBuf {
    let mut path = home_dir().expect("Error getting home path");
    path.push(SCILLA_CONFIG_RELATIVE_PATH);
    path
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ScillaConfig {
    pub rpc_url: String,
    pub commitment_level: CommitmentLevel,
    pub keypair_path: PathBuf,
    #[serde(default)]
    pub cluster: Option<String>,
}

impl ScillaConfig {
    pub fn load() -> Result<ScillaConfig, ScillaError> {
        let scilla_config_path = scilla_config_path();
        println!("{:?}", scilla_config_path);
        if !scilla_config_path.exists() {
            return Err(ScillaError::ConfigPathDoesntExists);
        }
        let data = fs::read_to_string(scilla_config_path)?;
        let config: ScillaConfig = toml::from_str(&data)?;
        Ok(config)
    }

    pub fn explorer_url(&self, signature: impl std::fmt::Display) -> String {
        Self::explorer_url_for_cluster(signature, self.cluster.as_deref())
    }

    pub fn explorer_url_for_cluster(
        signature: impl std::fmt::Display,
        cluster: Option<&str>,
    ) -> String {
        let cluster = cluster.unwrap_or("mainnet");
        if cluster.eq_ignore_ascii_case("mainnet") || cluster.eq_ignore_ascii_case("mainnet-beta")
        {
            format!("https://explorer.solana.com/tx/{signature}")
        } else {
            format!("https://explorer.solana.com/tx/{signature}?cluster={cluster}")
        }
    }
}

use {crate::commands::CommandFlow, thiserror::Error};

pub type ScillaResult = Result<CommandFlow, ScillaError>;

#[derive(Debug, Error)]
pub enum ScillaError {
    #[error("Scilla ScillaConfig path doesnt exists")]
    ConfigPathDoesNotExist,
    #[error("Io error")]
    IoError(#[from] std::io::Error),
    #[error("Toml Parse error")]
    TomlParseError(#[from] toml::de::Error),
    #[error("Anyhow err")]
    Anyhow(#[from] anyhow::Error),
}

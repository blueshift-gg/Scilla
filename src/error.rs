use {crate::commands::CommandExec, thiserror::Error};

pub type ScillaResult<T> = anyhow::Result<CommandExec<T>>;

#[derive(Debug, Error)]
pub enum ScillaError {
    #[error("Scilla ScillaConfig path doesnt exists")]
    ConfigPathDoesNotExist,
    #[error("Could not determine home directory. Please set the HOME environment variable.")]
    HomeDirectoryNotFound,
    #[error("Io error")]
    IoError(#[from] std::io::Error),
    #[error("Toml Parse error")]
    TomlParseError(#[from] toml::de::Error),
    #[error("Anyhow err")]
    Anyhow(#[from] anyhow::Error),
}

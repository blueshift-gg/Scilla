use {
    crate::{
        commands::CommandExec,
        config::{ScillaConfig, scilla_config_path},
        error::ScillaResult,
        prompt::prompt_data,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::{Confirm, Select},
    solana_commitment_config::CommitmentLevel,
    std::{fmt, fs, path::PathBuf},
};

/// Commands related to run a local validator and it's operations
#[derive(Debug, Clone)]
pub enum ValidatorCommand {
    Start,
    Stop,
    Status,
    Logs,
    Config,
    Exit,
}

impl ValidatorCommand {
    pub fn execute(&self) -> ScillaResult<()> {
        match self {
            ValidatorCommand::Start => {
	           	todo!()
            }
            ValidatorCommand::Stop => {
	           	todo!()
            }
            ValidatorCommand::Status => {
	           	todo!()
            }
            ValidatorCommand::Logs => {
	           	todo!()
            }
            ValidatorCommand::Config => {
	           	todo!()
            }
            ValidatorCommand::Exit => {
	           	todo!()
            }
        }
    }
}

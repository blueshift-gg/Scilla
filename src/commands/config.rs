use crate::{ScillaContext, ScillaResult, commands::CommandExec};
/// Commands related to configuration like RPC_URL , KEYAPAIR_PATH etc
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Generate,
    Edit,
    GoBack,
}

impl ConfigCommand {
    pub fn description(&self) -> &'static str {
        match self {
            ConfigCommand::Show => "Display current Scilla configuration",
            ConfigCommand::Generate => "Set up a new Scilla configuration",
            ConfigCommand::Edit => "Modify existing Scilla configuration",
            ConfigCommand::GoBack => "Go back",
        }
    }
}

impl ConfigCommand {
    pub async fn process_command(&self, _ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            ConfigCommand::Show => todo!(),
            ConfigCommand::Generate => todo!(),
            ConfigCommand::Edit => todo!(),
            ConfigCommand::GoBack => Ok(CommandExec::GoBack),
        }
    }
}

use crate::{commands::CommandExec, context::ScillaContext, error::ScillaResult};

/// Commands related to staking operations
#[derive(Debug, Clone)]
pub enum StakeCommand {
    Create,
    Delegate,
    Deactivate,
    Withdraw,
    Merge,
    Split,
    Show,
    History,
    GoBack,
}

impl StakeCommand {
    pub fn description(&self) -> &'static str {
        match self {
            StakeCommand::Create => "Create a new stake account",
            StakeCommand::Delegate => "Delegate stake to a validator",
            StakeCommand::Deactivate => "Begin stake cooldown",
            StakeCommand::Withdraw => "Withdraw SOL from deactivated stake",
            StakeCommand::Merge => "Combine two stake accounts",
            StakeCommand::Split => "Split stake into multiple accounts",
            StakeCommand::Show => "Display stake account details",
            StakeCommand::History => "View stake account history",
            StakeCommand::GoBack => "Go back",
        }
    }
}

impl StakeCommand {
    pub async fn process_command(&self, _ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            StakeCommand::Create => todo!(),
            StakeCommand::Delegate => todo!(),
            StakeCommand::Deactivate => todo!(),
            StakeCommand::Withdraw => todo!(),
            StakeCommand::Merge => todo!(),
            StakeCommand::Split => todo!(),
            StakeCommand::Show => todo!(),
            StakeCommand::History => todo!(),
            StakeCommand::GoBack => Ok(CommandExec::GoBack),
        }
    }
}

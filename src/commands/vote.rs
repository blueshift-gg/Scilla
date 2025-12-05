use crate::{ScillaContext, ScillaResult, commands::CommandExec};
/// Commands related to validator/vote account operations
#[derive(Debug, Clone)]
pub enum VoteCommand {
    CreateVoteAccount,
    AuthorizeVoter,
    WithdrawFromVote,
    ShowVoteAccount,
    GoBack,
}

impl VoteCommand {
    pub fn description(&self) -> &'static str {
        match self {
            VoteCommand::CreateVoteAccount => "Initialize a new vote account",
            VoteCommand::AuthorizeVoter => "Change authorized voter",
            VoteCommand::WithdrawFromVote => "Withdraw from vote account",
            VoteCommand::ShowVoteAccount => "Display vote account info",
            VoteCommand::GoBack => "Go back",
        }
    }
}

impl VoteCommand {
    pub async fn process_command(&self, _ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            VoteCommand::CreateVoteAccount => todo!(),
            VoteCommand::AuthorizeVoter => todo!(),
            VoteCommand::WithdrawFromVote => todo!(),
            VoteCommand::ShowVoteAccount => todo!(),
            VoteCommand::GoBack => Ok(CommandExec::GoBack),
        }
    }
}

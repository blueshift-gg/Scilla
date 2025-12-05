use {
    crate::{
        commands::{
            account::AccountCommand, cluster::ClusterCommand, config::ConfigCommand,
            stake::StakeCommand, vote::VoteCommand,
        },
        context::ScillaContext,
        error::ScillaResult,
    },
    console::style,
    std::process::{ExitCode, Termination},
};

pub mod account;
pub mod cluster;
pub mod config;
pub mod stake;
pub mod vote;

pub enum CommandExec<T> {
    Process(T),
    GoBack,
    Exit,
}

impl<T> Termination for CommandExec<T> {
    fn report(self) -> std::process::ExitCode {
        println!("{}", style("Goodbye 👋").dim());
        ExitCode::SUCCESS
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Cluster(ClusterCommand),
    Stake(StakeCommand),
    Account(AccountCommand),
    Vote(VoteCommand),
    ScillaConfig(ConfigCommand),
    Exit,
}

impl Command {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            Command::Cluster(cluster_command) => cluster_command.process_command(ctx).await,
            Command::Stake(stake_command) => stake_command.process_command(ctx).await,
            Command::Account(account_command) => account_command.process_command(ctx).await,
            Command::Vote(_vote_command) => todo!(),
            Command::ScillaConfig(_config_command) => todo!(),
            Command::Exit => Ok(CommandExec::Exit),
        }
    }
}

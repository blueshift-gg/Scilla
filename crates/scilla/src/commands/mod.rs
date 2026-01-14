use {
    crate::{
        commands::{
            account::AccountCommand, cluster::ClusterCommand, config::ConfigCommand,
            stake::StakeCommand, transaction::TransactionCommand, vote::VoteCommand,
        },
        context::ScillaContext,
    },
    console::style,
    std::{
        fmt,
        process::{ExitCode, Termination},
    },
};

pub mod account;
pub mod cluster;
pub mod config;
pub mod program;
pub mod stake;
pub mod transaction;
pub mod vote;

pub enum CommandFlow {
    Processed,
    NavigateTo(NavigationTarget),
    Exit,
}

pub enum NavigationTarget {
    MainMenu,
    PreviousSection,
}

impl Termination for CommandFlow {
    fn report(self) -> std::process::ExitCode {
        println!("{}", style("Goodbye 👋").dim());
        ExitCode::SUCCESS
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Cluster(ClusterCommand),
    Stake(StakeCommand),
    Account(AccountCommand),
    Vote(VoteCommand),
    Transaction(TransactionCommand),
    ScillaConfig(ConfigCommand),
    Exit,
}

impl Command {
    pub async fn process_command(&self, ctx: &mut ScillaContext) -> CommandFlow {
        match self {
            Command::Cluster(cluster_command) => cluster_command.process_command(ctx).await,
            Command::Stake(stake_command) => stake_command.process_command(ctx).await,
            Command::Account(account_command) => account_command.process_command(ctx).await,
            Command::Vote(vote_command) => vote_command.process_command(ctx).await,
            Command::Transaction(transaction_command) => {
                transaction_command.process_command(ctx).await
            }
            Command::ScillaConfig(config_command) => config_command.process_command(ctx),
            Command::Exit => CommandFlow::Exit,
        }
    }

    /// Returns the section (CommandGroup) this command belongs to
    pub fn section(&self) -> CommandGroup {
        match self {
            Command::Cluster(_) => CommandGroup::Cluster,
            Command::Stake(_) => CommandGroup::Stake,
            Command::Account(_) => CommandGroup::Account,
            Command::Vote(_) => CommandGroup::Vote,
            Command::Transaction(_) => CommandGroup::Transaction,
            Command::ScillaConfig(_) => CommandGroup::ScillaConfig,
            Command::Exit => CommandGroup::Exit,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CommandGroup {
    Account,
    Cluster,
    Stake,
    Vote,
    Transaction,
    ScillaConfig,
    Exit,
}

impl fmt::Display for CommandGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            CommandGroup::Account => "Account",
            CommandGroup::Cluster => "Cluster",
            CommandGroup::Stake => "Stake",
            CommandGroup::Vote => "Vote",
            CommandGroup::Transaction => "Transaction",
            CommandGroup::ScillaConfig => "ScillaConfig",
            CommandGroup::Exit => "Exit",
        };
        write!(f, "{command}")
    }
}

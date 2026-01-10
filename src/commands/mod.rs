use {
    crate::{
        commands::{
            account::AccountCommand, cluster::ClusterCommand, config::ConfigCommand,
            program::ProgramCommand, stake::StakeCommand, transaction::TransactionCommand,
            vote::VoteCommand,
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

pub enum CommandFlow<T> {
    Process(T),
    GoBack,
    Exit,
}

impl<T> Termination for CommandFlow<T> {
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
    Program(ProgramCommand),
    Vote(VoteCommand),
    Transaction(TransactionCommand),
    ScillaConfig(ConfigCommand),
    Exit,
}

impl Command {
    pub async fn process_command(&self, ctx: &mut ScillaContext) -> CommandFlow<()> {
        match self {
            Command::Cluster(cluster_command) => cluster_command.process_command(ctx).await,
            Command::Stake(stake_command) => stake_command.process_command(ctx).await,
            Command::Account(account_command) => account_command.process_command(ctx).await,
            Command::Program(program_command) => program_command.process_command(ctx).await,
            Command::Vote(vote_command) => vote_command.process_command(ctx).await,
            Command::Transaction(transaction_command) => {
                transaction_command.process_command(ctx).await
            }
            Command::ScillaConfig(config_command) => config_command.process_command(ctx),
            Command::Exit => CommandFlow::Exit,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandGroup {
    Account,
    Program,
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
            CommandGroup::Program => "Program",
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

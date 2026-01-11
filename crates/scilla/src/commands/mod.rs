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
    std::process::{ExitCode, Termination},
};

pub mod account;
pub mod cluster;
pub mod config;
pub mod main_command;
pub mod navigation;
pub mod program;
pub mod stake;
pub mod transaction;
pub mod vote;

pub enum CommandFlow {
    Processed,
    NavigateTo(NavigationTarget),
    Exit,
}

impl Termination for CommandFlow {
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
    Program(ProgramCommand),
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
            Command::Vote(vote_command) => vote_command.process_command(ctx).await,
            Command::Program(program_command) => program_command.process_command(ctx).await,
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
    Cluster,
    Stake,
    Vote,
    Program,
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
            CommandGroup::Program => "Program",
            CommandGroup::Transaction => "Transaction",
            CommandGroup::ScillaConfig => "ScillaConfig",
            CommandGroup::Exit => "Exit",
        };
        write!(f, "{command}")
    }
}

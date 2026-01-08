use {
    crate::{
        commands::{
            Command, CommandGroup, account::AccountCommand, cluster::ClusterCommand,
            config::ConfigCommand, stake::StakeCommand, transaction::TransactionCommand,
            vote::VoteCommand,
        },
        context::ScillaContext,
        ui::print_error,
    },
    console::style,
    inquire::{Confirm, InquireError, Select, Text},
    std::{fmt::Display, path::PathBuf, process::exit, str::FromStr},
};
pub fn prompt_for_command() -> anyhow::Result<Command> {
    let top_level = Select::new(
        "Choose a command group:",
        vec![
            CommandGroup::Account,
            CommandGroup::Cluster,
            CommandGroup::Stake,
            CommandGroup::Vote,
            CommandGroup::Transaction,
            CommandGroup::ScillaConfig,
            CommandGroup::Exit,
        ],
    )
    .prompt()?;

    let command = match top_level {
        CommandGroup::Cluster => Command::Cluster(prompt_cluster()?),
        CommandGroup::Stake => Command::Stake(prompt_stake()?),
        CommandGroup::Account => Command::Account(prompt_account()?),
        CommandGroup::Vote => Command::Vote(prompt_vote()?),
        CommandGroup::ScillaConfig => Command::ScillaConfig(prompt_config()?),
        CommandGroup::Transaction => Command::Transaction(prompt_transaction()?),
        CommandGroup::Exit => Command::Exit,
    };

    Ok(command)
}

fn prompt_cluster() -> anyhow::Result<ClusterCommand> {
    let choice = Select::new(
        "Cluster Command:",
        vec![
            ClusterCommand::EpochInfo,
            ClusterCommand::CurrentSlot,
            ClusterCommand::BlockHeight,
            ClusterCommand::BlockTime,
            ClusterCommand::Validators,
            ClusterCommand::ClusterVersion,
            ClusterCommand::SupplyInfo,
            ClusterCommand::Inflation,
            ClusterCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

fn prompt_stake() -> anyhow::Result<StakeCommand> {
    let choice = Select::new(
        "Stake Command:",
        vec![
            StakeCommand::Create,
            StakeCommand::Delegate,
            StakeCommand::Deactivate,
            StakeCommand::Withdraw,
            StakeCommand::Merge,
            StakeCommand::Split,
            StakeCommand::Show,
            StakeCommand::History,
            StakeCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

fn prompt_account() -> anyhow::Result<AccountCommand> {
    let choice = Select::new(
        "Account Command:",
        vec![
            AccountCommand::FetchAccount,
            AccountCommand::Balance,
            AccountCommand::Transfer,
            AccountCommand::Airdrop,
            AccountCommand::LargestAccounts,
            AccountCommand::NonceAccount,
            AccountCommand::Rent,
            AccountCommand::GoBack,
        ],
    )
    .with_page_size(10)
    .prompt()?;

    Ok(choice)
}

fn prompt_vote() -> anyhow::Result<VoteCommand> {
    let choice = Select::new(
        "Vote Command:",
        vec![
            VoteCommand::CreateVoteAccount,
            VoteCommand::AuthorizeVoter,
            VoteCommand::WithdrawFromVoteAccount,
            VoteCommand::ShowVoteAccount,
            VoteCommand::CloseVoteAccount,
            VoteCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

fn prompt_transaction() -> anyhow::Result<TransactionCommand> {
    let choice = Select::new(
        "Transaction Command:",
        vec![
            TransactionCommand::CheckConfirmation,
            TransactionCommand::FetchStatus,
            TransactionCommand::FetchTransaction,
            TransactionCommand::SendTransaction,
            TransactionCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

fn prompt_config() -> anyhow::Result<ConfigCommand> {
    let choice = Select::new(
        "ScillaConfig Command:",
        vec![
            ConfigCommand::Show,
            ConfigCommand::Edit,
            ConfigCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

pub fn prompt_input_data<T>(msg: &str) -> T
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    loop {
        let input = match Text::new(msg).prompt() {
            Ok(v) => v,
            Err(e) => match e {
                InquireError::OperationInterrupted | InquireError::OperationCanceled => {
                    println!("{}", style("Operation cancelled. Exiting.").yellow().bold());
                    exit(0);
                }
                _ => {
                    print_error(format!("Invalid input: {e}. Please try again."));
                    continue;
                }
            },
        };

        match input.parse::<T>() {
            Ok(value) => return value,
            Err(e) => print_error(format!("Parse error : {e}. Please try again.")),
        }
    }
}

pub fn prompt_select_data<T>(msg: &str, options: Vec<T>) -> T
where
    T: Display + Clone,
{
    loop {
        match Select::new(msg, options.clone()).prompt() {
            Ok(v) => return v,
            Err(e) => match e {
                InquireError::OperationInterrupted | InquireError::OperationCanceled => {
                    println!("{}", style("Operation cancelled. Exiting.").yellow().bold());
                    exit(0);
                }
                _ => {
                    print_error(format!("Invalid Choice: {e}. Please try again."));
                    continue;
                }
            },
        }
    }
}

pub fn prompt_keypair_path(msg: &str, ctx: &ScillaContext) -> PathBuf {
    let default_path = ctx.keypair_path().display().to_string();

    loop {
        let input = match Text::new(msg)
            .with_default(&default_path)
            .with_help_message("Press Enter to use the default keypair")
            .prompt()
        {
            Ok(v) => v,
            Err(e) => match e {
                InquireError::OperationInterrupted | InquireError::OperationCanceled => {
                    println!("{}", style("Operation cancelled. Exiting.").yellow().bold());
                    exit(0);
                }
                _ => {
                    print_error(format!("Invalid input: {e}. Please try again."));
                    continue;
                }
            },
        };

        let input = if input.trim().is_empty() {
            &default_path
        } else {
            &input
        };

        match PathBuf::from_str(input) {
            Ok(value) => return value,
            Err(e) => {
                print_error(format!("Invalid path: {e}. Please try again."));
            }
        }
    }
}

pub fn prompt_confirmation(msg: &str) -> bool {
    Confirm::new(msg).prompt().unwrap_or(false)
}

use {
    crate::commands::{
        Command, CommandGroup, account::AccountCommand, cluster::ClusterCommand,
        config::ConfigCommand, stake::StakeCommand, transaction::TransactionCommand,
        vote::VoteCommand, token::TokenCommand,
    },
    inquire::{Select, Text},
    std::str::FromStr,
};
pub fn prompt_for_command() -> anyhow::Result<Command> {
    let top_level = Select::new(
        "Choose a command group:",
        vec![
            CommandGroup::Account,
            CommandGroup::Token,
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
        CommandGroup::Token => Command::Token(prompt_token()?),
        CommandGroup::Account => Command::Account(prompt_account()?),
        CommandGroup::Vote => Command::Vote(prompt_vote()?),
        CommandGroup::ScillaConfig => Command::ScillaConfig(prompt_config()?),
        CommandGroup::Transaction => Command::Transaction(prompt_transaction()?),
        CommandGroup::Exit => Command::Exit,
    };

    Ok(command)
}

fn prompt_token() -> anyhow::Result<TokenCommand> {
    let choice = Select::new(
        "Token Command:",
        vec![
            TokenCommand::ListTokenAccounts,
            TokenCommand::TokenBalance,
            TokenCommand::MintInfo,
            TokenCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
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
            AccountCommand::GoBack,
        ],
    )
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
            ConfigCommand::Generate,
            ConfigCommand::Edit,
            ConfigCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

pub fn prompt_data<T>(msg: &str) -> anyhow::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: ToString + Send + Sync + 'static,
{
    loop {
        let input = Text::new(msg).prompt()?;
        match T::from_str(&input) {
            Ok(value) => return Ok(value),
            Err(e) => {
                eprintln!("Invalid input: {}. Please try again.\n", e.to_string());
            }
        }
    }
}

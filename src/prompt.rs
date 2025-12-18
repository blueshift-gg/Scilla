use {
    crate::commands::{
        Command, account::AccountCommand, cluster::ClusterCommand, config::ConfigCommand,
        stake::StakeCommand, vote::VoteCommand,
    },
    inquire::{Select, Text},
    std::str::FromStr,
};

pub fn prompt_for_command() -> anyhow::Result<Command> {
    let top_level = Select::new(
        "Choose a command group:",
        vec![
            "Account",
            "Cluster",
            "Stake",
            "Vote",
            "ScillaConfig",
            "Exit",
        ],
    )
    .prompt()?;

    let command = match top_level {
        "Cluster" => Command::Cluster(prompt_cluster()?),
        "Stake" => Command::Stake(prompt_stake()?),
        "Account" => Command::Account(prompt_account()?),
        "Vote" => Command::Vote(prompt_vote()?),
        "ScillaConfig" => Command::ScillaConfig(prompt_config()?),
        "Exit" => Command::Exit,
        _ => unreachable!(),
    };

    Ok(command)
}

fn prompt_cluster() -> anyhow::Result<ClusterCommand> {
    let choice = Select::new(
        "Cluster Command:",
        vec![
            "Epoch Info",
            "Current Slot",
            "Block Height",
            "Block Time",
            "Validators",
            "Cluster Version",
            "Supply Info",
            "Inflation",
            "Go Back",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Epoch Info" => ClusterCommand::EpochInfo,
        "Current Slot" => ClusterCommand::CurrentSlot,
        "Block Height" => ClusterCommand::BlockHeight,
        "Block Time" => ClusterCommand::BlockTime,
        "Validators" => ClusterCommand::Validators,
        "Cluster Version" => ClusterCommand::ClusterVersion,
        "Supply Info" => ClusterCommand::SupplyInfo,
        "Inflation" => ClusterCommand::Inflation,
        "Go Back" => ClusterCommand::GoBack,
        _ => unreachable!(),
    })
}

fn prompt_stake() -> anyhow::Result<StakeCommand> {
    let choice = Select::new(
        "Stake Command:",
        vec![
            "Create",
            "Delegate",
            "Deactivate",
            "Withdraw",
            "Merge",
            "Split",
            "Show",
            "History",
            "Go Back",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Create" => StakeCommand::Create,
        "Delegate" => StakeCommand::Delegate,
        "Deactivate" => StakeCommand::Deactivate,
        "Withdraw" => StakeCommand::Withdraw,
        "Merge" => StakeCommand::Merge,
        "Split" => StakeCommand::Split,
        "Show" => StakeCommand::Show,
        "History" => StakeCommand::History,
        "Go Back" => StakeCommand::GoBack,
        _ => unreachable!(),
    })
}

fn prompt_account() -> anyhow::Result<AccountCommand> {
    let choice = Select::new(
        "Account Command:",
        vec![
            "Fetch Account",
            "Balance",
            "Transfer",
            "Airdrop",
            "Check Transaction Confirmation",
            "Largest Accounts",
            "Nonce Account",
            "Go Back",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Fetch Account" => AccountCommand::FetchAccount,
        "Balance" => AccountCommand::Balance,
        "Transfer" => AccountCommand::Transfer,
        "Airdrop" => AccountCommand::Airdrop,
        "Confirm Transaction" => AccountCommand::CheckTransactionConfirmation,
        "Largest Accounts" => AccountCommand::LargestAccounts,
        "Nonce Account" => AccountCommand::NonceAccount,
        "Go Back" => AccountCommand::GoBack,
        _ => unreachable!(),
    })
}

fn prompt_vote() -> anyhow::Result<VoteCommand> {
    let choice = Select::new(
        "Vote Command:",
        vec![
            "Create Vote Account",
            "Authorize Voter",
            "Withdraw From Vote Account",
            "Show Vote Account",
            "Go Back",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Create Vote Account" => VoteCommand::CreateVoteAccount,
        "Authorize Voter" => VoteCommand::AuthorizeVoter,
        "Withdraw From Vote Account" => VoteCommand::WithdrawFromVoteAccount,
        "Show Vote Account" => VoteCommand::ShowVoteAccount,
        "Go Back" => VoteCommand::GoBack,
        _ => unreachable!(),
    })
}

fn prompt_config() -> anyhow::Result<ConfigCommand> {
    let choice = Select::new(
        "ScillaConfig Command:",
        vec![
            "Show ScillaConfig",
            "Generate ScillaConfig",
            "Edit ScillaConfig",
            "Go Back",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Show ScillaConfig" => ConfigCommand::Show,
        "Generate ScillaConfig" => ConfigCommand::Generate,
        "Edit ScillaConfig" => ConfigCommand::Edit,
        "Go Back" => ConfigCommand::GoBack,
        _ => unreachable!(),
    })
}

pub fn prompt_data<T>(msg: &str) -> anyhow::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: ToString + Send + Sync + 'static,
{
    let input = Text::new(msg).prompt()?;
    T::from_str(&input).map_err(|e| anyhow::anyhow!(e.to_string()))
}

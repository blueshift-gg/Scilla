use {
    crate::{
        commands::{
            Command,
            account::AccountCommand,
            address::AddressCommand,
            cluster::ClusterCommand,
            config::ConfigCommand,
            main_command::MainCommand,
            navigation::NavigationTarget,
            program::{ProgramCommand, ProgramShared},
            stake::StakeCommand,
            transaction::TransactionCommand,
            vote::VoteCommand,
        },
        constants::{DEVNET_RPC, MAINNET_RPC, TESTNET_RPC},
        context::ScillaContext,
        ui::print_error,
    },
    console::style,
    inquire::{Confirm, InquireError, Select, Text},
    solana_transaction_status::UiTransactionEncoding,
    std::{fmt::Display, path::PathBuf, process::exit, str::FromStr},
};
pub fn prompt_main_section() -> anyhow::Result<impl Command> {
    let command = Select::new(
        "Choose a command group:",
        vec![
            MainCommand::Account,
            MainCommand::Cluster,
            MainCommand::Stake,
            MainCommand::Program,
            MainCommand::Vote,
            MainCommand::Transaction,
            MainCommand::ScillaConfig,
            MainCommand::Address,
            MainCommand::Exit,
        ],
    )
    .prompt()?;

    Ok(command)
}

pub fn prompt_account_section() -> anyhow::Result<AccountCommand> {
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

pub fn prompt_cluster_section() -> anyhow::Result<ClusterCommand> {
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

pub fn prompt_stake_section() -> anyhow::Result<StakeCommand> {
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

pub fn prompt_program_section() -> anyhow::Result<ProgramCommand> {
    let choice = Select::new(
        "Program Command:",
        vec![
            ProgramCommand::ProgramLegacy,
            ProgramCommand::ProgramV4,
            ProgramCommand::GoBack,
        ],
    )
    .with_page_size(10)
    .prompt()?;

    Ok(choice)
}

pub fn prompt_program_section_shared() -> anyhow::Result<ProgramShared> {
    let choice = Select::new(
        "Program Action:",
        vec![
            ProgramShared::Deploy,
            ProgramShared::Upgrade,
            ProgramShared::Build,
            ProgramShared::Close,
            ProgramShared::Extend,
            ProgramShared::GoBack,
        ],
    )
    .with_page_size(10)
    .prompt()?;

    Ok(choice)
}

pub fn prompt_vote_section() -> anyhow::Result<VoteCommand> {
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

pub fn prompt_transaction_section() -> anyhow::Result<TransactionCommand> {
    let choice = Select::new(
        "Transaction Command:",
        vec![
            TransactionCommand::CheckConfirmation,
            TransactionCommand::FetchStatus,
            TransactionCommand::FetchTransaction,
            TransactionCommand::SendTransaction,
            TransactionCommand::SimulateTransaction,
            TransactionCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

pub fn prompt_address_section() -> anyhow::Result<AddressCommand> {
    let choice = Select::new(
        "Address Command:",
        vec![
            AddressCommand::Address,
            AddressCommand::DerivePda,
            AddressCommand::GoBack,
        ],
    )
    .prompt()?;

    Ok(choice)
}

pub fn prompt_config_section() -> anyhow::Result<ConfigCommand> {
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

        let input = input.trim();

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "Mainnet"),
            Network::Testnet => write!(f, "Testnet"),
            Network::Devnet => write!(f, "Devnet"),
        }
    }
}

impl Network {
    fn rpc_url(&self) -> &'static str {
        match self {
            Network::Mainnet => MAINNET_RPC,
            Network::Testnet => TESTNET_RPC,
            Network::Devnet => DEVNET_RPC,
        }
    }

    fn all() -> Vec<Network> {
        vec![Network::Mainnet, Network::Testnet, Network::Devnet]
    }
}

pub fn prompt_network_rpc_url() -> anyhow::Result<String> {
    let network = Select::new("Select network:", Network::all()).prompt()?;
    Ok(network.rpc_url().to_string())
}

pub fn prompt_go_back() -> NavigationTarget {
    let choice = Select::new(
        "Go Back to menu or last section",
        vec!["Main Section", "Previous Section"],
    )
    .prompt()
    .unwrap();
    match choice {
        "Main Section" => NavigationTarget::MainSection,
        "Previous Section" => NavigationTarget::PreviousSection,
        _ => unreachable!(),
    }
}

pub fn prompt_encoding_options() -> UiTransactionEncoding {
    prompt_select_data(
        "Select encoding format:",
        vec![
            UiTransactionEncoding::Base64,
            UiTransactionEncoding::Base58,
            UiTransactionEncoding::Binary,
            UiTransactionEncoding::Json,
            UiTransactionEncoding::JsonParsed,
        ],
    )
}

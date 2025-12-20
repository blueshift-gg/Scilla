use {
    crate::{
        commands::CommandExec,
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::lamports_to_sol,
        prompt::prompt_data,
        ui::{print_error, show_spinner},
    },
    anyhow::bail,
    comfy_table::{presets::UTF8_FULL, Cell, Table},
    console::style,
    inquire::Select,
    solana_nonce::versions::Versions,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::config::{RpcLargestAccountsConfig, RpcLargestAccountsFilter},
    solana_signature::Signature,
    std::fmt,
};

/// Commands related to wallet or account management
#[derive(Debug, Clone)]
pub enum AccountCommand {
    FetchAccount,
    Balance,
    Transfer,
    Airdrop,
    CheckTransactionConfirmation,
    LargestAccounts,
    NonceAccount,
    GoBack,
}

impl AccountCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            AccountCommand::FetchAccount => "Fetching account…",
            AccountCommand::Balance => "Checking SOL balance…",
            AccountCommand::Transfer => "Sending SOL to another wallet…",
            AccountCommand::Airdrop => "Requesting SOL on devnet/testnet…",
            AccountCommand::CheckTransactionConfirmation => "Checking transaction confirmation…",
            AccountCommand::LargestAccounts => "Fetching largest accounts on the cluster…",
            AccountCommand::NonceAccount => "Inspecting or managing durable nonces…",
            AccountCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for AccountCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            AccountCommand::FetchAccount => "Fetch Account",
            AccountCommand::Balance => "Balance",
            AccountCommand::Transfer => "Transfer",
            AccountCommand::Airdrop => "Airdrop",
            AccountCommand::CheckTransactionConfirmation => "Check Transaction Confirmation",
            AccountCommand::LargestAccounts => "Largest Accounts",
            AccountCommand::NonceAccount => "Nonce Account",
            AccountCommand::GoBack => "Go Back",
        };
        write!(f, "{command}")
    }
}

impl AccountCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            AccountCommand::FetchAccount => {
                let pubkey: Pubkey = prompt_data("Enter Pubkey:")?;
                show_spinner(self.spinner_msg(), fetch_acc_data(ctx, &pubkey)).await?;
            }
            AccountCommand::Balance => {
                let pubkey: Pubkey = prompt_data("Enter Pubkey :")?;
                show_spinner(self.spinner_msg(), fetch_account_balance(ctx, &pubkey)).await?;
            }
            AccountCommand::Transfer => {
                // show_spinner(self.spinner_msg(), todo!()).await?;
            }
            AccountCommand::Airdrop => {
                show_spinner(self.spinner_msg(), request_sol_airdrop(ctx)).await?;
            }
            AccountCommand::CheckTransactionConfirmation => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(self.spinner_msg(), confirm_transaction(ctx, &signature)).await?;
            }
            AccountCommand::LargestAccounts => {
                show_spinner(self.spinner_msg(), fetch_largest_accounts(ctx)).await?;
            }
            AccountCommand::NonceAccount => {
                let pubkey: Pubkey = prompt_data("Enter nonce account pubkey:")?;
                show_spinner(self.spinner_msg(), fetch_nonce_account(ctx, &pubkey)).await?;
            }
            AccountCommand::GoBack => {
                return Ok(CommandExec::GoBack);
            }
        }

        Ok(CommandExec::Process(()))
    }
}

async fn request_sol_airdrop(ctx: &ScillaContext) -> anyhow::Result<()> {
    let sig = ctx.rpc().request_airdrop(ctx.pubkey(), 1).await;
    match sig {
        Ok(signature) => {
            println!(
                "{} {}",
                style("Airdrop requested successfully!").green().bold(),
                style(format!("Signature: {signature}")).cyan()
            );
        }
        Err(err) => {
            print_error(format!("Airdrop failed: {err}"));
        }
    }

    Ok(())
}

async fn fetch_acc_data(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let acc = ctx.rpc().get_account(pubkey).await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Lamports"),
            Cell::new(format!("{}", acc.lamports)),
        ])
        .add_row(vec![
            Cell::new("Data Length"),
            Cell::new(format!("{}", acc.data.len())),
        ])
        .add_row(vec![
            Cell::new("Owner"),
            Cell::new(format!("{}", acc.owner)),
        ])
        .add_row(vec![
            Cell::new("Executable"),
            Cell::new(format!("{}", acc.executable)),
        ])
        .add_row(vec![
            Cell::new("Rent Epoch"),
            Cell::new(format!("{}", acc.rent_epoch)),
        ]);

    println!("{}\n{}", style("ACCOUNT INFO").green().bold(), table);

    Ok(())
}

async fn fetch_account_balance(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let acc = ctx.rpc().get_account(pubkey).await?;
    let acc_balance = lamports_to_sol(acc.lamports);

    println!(
        "{}\n{}",
        style("Account balance in SOL:").green().bold(),
        style(format!("{acc_balance:#?}")).cyan()
    );

    Ok(())
}

async fn confirm_transaction(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()> {
    let confirmed = ctx.rpc().confirm_transaction(signature).await?;

    let status = if confirmed {
        "Confirmed"
    } else {
        "Not Confirmed"
    };
    let status_color = if confirmed {
        style(status).green()
    } else {
        style(status).yellow()
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Signature"),
            Cell::new(signature.to_string()),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new(status_color.to_string()),
        ]);

    println!("\n{}", style("TRANSACTION CONFIRMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_largest_accounts(ctx: &ScillaContext) -> anyhow::Result<()> {
    let filter_choice = Select::new(
        "Filter accounts by:",
        vec!["All", "Circulating", "Non-Circulating"],
    )
    .prompt()?;

    let filter = match filter_choice {
        "Circulating" => Some(RpcLargestAccountsFilter::Circulating),
        "Non-Circulating" => Some(RpcLargestAccountsFilter::NonCirculating),
        _ => None,
    };

    let config = RpcLargestAccountsConfig {
        commitment: Some(ctx.rpc().commitment()),
        filter,
        sort_results: Some(true),
    };

    let response = ctx.rpc().get_largest_accounts_with_config(config).await?;
    let largest_accounts = response.value;

    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("#").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Address").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Balance (SOL)").add_attribute(comfy_table::Attribute::Bold),
    ]);

    for (idx, account) in largest_accounts.iter().enumerate() {
        let balance_sol = lamports_to_sol(account.lamports);
        table.add_row(vec![
            Cell::new(format!("{}", idx + 1)),
            Cell::new(account.address.clone()),
            Cell::new(format!("{balance_sol:.2}")),
        ]);
    }

    println!("\n{}", style("LARGEST ACCOUNTS").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_nonce_account(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let account = ctx.rpc().get_account(pubkey).await?;

    let versions = bincode::deserialize::<Versions>(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize nonce account data: {e}"))?;

    let solana_nonce::state::State::Initialized(data) = versions.state() else {
        bail!("This account is not an initialized nonce account");
    };
    let data = data.clone();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Address"), Cell::new(pubkey.to_string())])
        .add_row(vec![
            Cell::new("Lamports"),
            Cell::new(format!("{}", account.lamports)),
        ])
        .add_row(vec![
            Cell::new("Balance (SOL)"),
            Cell::new(format!("{:.6}", lamports_to_sol(account.lamports))),
        ])
        .add_row(vec![
            Cell::new("Owner"),
            Cell::new(account.owner.to_string()),
        ])
        .add_row(vec![
            Cell::new("Executable"),
            Cell::new(format!("{}", account.executable)),
        ])
        .add_row(vec![
            Cell::new("Rent Epoch"),
            Cell::new(format!("{}", account.rent_epoch)),
        ])
        .add_row(vec![
            Cell::new("Nonce blockhash"),
            Cell::new(data.blockhash().to_string()),
        ])
        .add_row(vec![
            Cell::new("Authority"),
            Cell::new(data.authority.to_string()),
        ]);

    println!("\n{}", style("NONCE ACCOUNT INFO").green().bold());
    println!("{table}");

    Ok(())
}

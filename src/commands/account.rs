use {
    crate::{
        commands::CommandFlow,
        context::ScillaContext,
        misc::helpers::{bincode_deserialize, build_and_send_tx, lamports_to_sol, sol_to_lamports},
        prompt::prompt_input_data,
        ui::{print_error, show_spinner},
    },
    anyhow::bail,
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::Select,
    solana_nonce::versions::Versions,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::config::{RpcLargestAccountsConfig, RpcLargestAccountsFilter},
    solana_system_interface::instruction::transfer,
    std::fmt,
};

/// Commands related to wallet or account management
#[derive(Debug, Clone)]
pub enum AccountCommand {
    FetchAccount,
    Balance,
    Transfer,
    Airdrop,
    LargestAccounts,
    NonceAccount,
    Rent,
    GoBack,
}

impl AccountCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            AccountCommand::FetchAccount => "Fetching account…",
            AccountCommand::Balance => "Checking SOL balance…",
            AccountCommand::Transfer => "Sending SOL…",
            AccountCommand::Airdrop => "Requesting SOL on devnet/testnet…",
            AccountCommand::LargestAccounts => "Fetching largest accounts on the cluster…",
            AccountCommand::NonceAccount => "Inspecting or managing durable nonces…",
            AccountCommand::Rent => "Checking rent…",
            AccountCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for AccountCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            AccountCommand::FetchAccount => "Fetch account",
            AccountCommand::Balance => "Check balance",
            AccountCommand::Transfer => "Transfer SOL",
            AccountCommand::Airdrop => "Request airdrop",
            AccountCommand::LargestAccounts => "View largest accounts",
            AccountCommand::NonceAccount => "View nonce account",
            AccountCommand::Rent => "Check rent",
            AccountCommand::GoBack => "Go back",
        };
        write!(f, "{command}")
    }
}

impl AccountCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> CommandFlow<()> {
        match self {
            AccountCommand::FetchAccount => {
                let pubkey: Pubkey = prompt_input_data("Enter Pubkey:");
                show_spinner(self.spinner_msg(), fetch_acc_data(ctx, &pubkey)).await;
            }
            AccountCommand::Balance => {
                let pubkey: Pubkey = prompt_input_data("Enter Pubkey :");
                show_spinner(self.spinner_msg(), fetch_account_balance(ctx, &pubkey)).await;
            }
            AccountCommand::Transfer => {
                let to: Pubkey = prompt_input_data("Enter recipient Pubkey:");
                let amount: f64 = prompt_input_data("Enter amount (SOL):");
                show_spinner(self.spinner_msg(), transfer_sol(ctx, to, amount)).await;
            }
            AccountCommand::Airdrop => {
                show_spinner(self.spinner_msg(), request_sol_airdrop(ctx)).await;
            }
            AccountCommand::LargestAccounts => {
                show_spinner(self.spinner_msg(), fetch_largest_accounts(ctx)).await;
            }
            AccountCommand::NonceAccount => {
                let pubkey: Pubkey = prompt_input_data("Enter nonce account pubkey:");
                show_spinner(self.spinner_msg(), fetch_nonce_account(ctx, &pubkey)).await;
            }
            AccountCommand::Rent => {
                // get the rent for data bytes used in account
                let bytes: usize = prompt_input_data("Enter data size in bytes:");
                show_spinner(self.spinner_msg(), fetch_rent(ctx, bytes)).await;
            }
            AccountCommand::GoBack => {
                return CommandFlow::GoBack;
            }
        }

        CommandFlow::Process(())
    }
}

async fn request_sol_airdrop(ctx: &ScillaContext) -> anyhow::Result<()> {
    // request an airdrop worth of 1 SOL
    let sig = ctx
        .rpc()
        .request_airdrop(ctx.pubkey(), sol_to_lamports(1.0))
        .await;
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
            Cell::new("Field")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
            Cell::new("Value")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
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
        "{} {}",
        style("Account balance in SOL:").green().bold(),
        style(format!("{acc_balance:#?}")).cyan()
    );

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
            Cell::new(&account.address),
            Cell::new(format!("{balance_sol:.2}")),
        ]);
    }

    println!("\n{}", style("LARGEST ACCOUNTS").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_nonce_account(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let account = ctx.rpc().get_account(pubkey).await?;

    let versions = bincode_deserialize::<Versions>(&account.data, "nonce account data")?;

    let solana_nonce::state::State::Initialized(data) = versions.state() else {
        bail!("This account is not an initialized nonce account");
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
            Cell::new("Value")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
        ])
        .add_row(vec![Cell::new("Address"), Cell::new(pubkey)])
        .add_row(vec![
            Cell::new("Lamports"),
            Cell::new(format!("{}", account.lamports)),
        ])
        .add_row(vec![
            Cell::new("Balance (SOL)"),
            Cell::new(format!("{:.6}", lamports_to_sol(account.lamports))),
        ])
        .add_row(vec![Cell::new("Owner"), Cell::new(account.owner)])
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
            Cell::new(data.blockhash()),
        ])
        .add_row(vec![Cell::new("Authority"), Cell::new(data.authority)]);

    println!("\n{}", style("NONCE ACCOUNT INFO").green().bold());
    println!("{table}");

    Ok(())
}

async fn transfer_sol(
    ctx: &ScillaContext,
    receiver: Pubkey,
    amount_sol: f64,
) -> anyhow::Result<()> {
    let lamports = sol_to_lamports(amount_sol);

    // Validate transfer amount
    let balance = ctx.rpc().get_balance(ctx.pubkey()).await?;
    if lamports > balance {
        bail!(
            "Insufficient balance. You have {} SOL but tried to send {} SOL",
            lamports_to_sol(balance),
            amount_sol
        );
    }

    let instruction = transfer(ctx.pubkey(), &receiver, lamports);
    let signature = build_and_send_tx(ctx, &[instruction], &[ctx.keypair()]).await?;

    println!(
        "\n{} {}\n{}\n{}",
        style("Transfer successful!").green().bold(),
        style(format!("Amount: {} SOL", amount_sol)).cyan(),
        style(format!("Signature: {}", signature)).yellow(),
        style(format!("Recipient Address: {}", receiver)).yellow()
    );

    Ok(())
}

async fn fetch_rent(ctx: &ScillaContext, bytes: usize) -> anyhow::Result<()> {
    let min_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(bytes)
        .await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
            Cell::new("Value")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Data Size"),
            Cell::new(format!("{} bytes", bytes)),
        ])
        .add_row(vec![
            Cell::new("Minimum Balance (lamports)"),
            Cell::new(format!("{}", min_balance)),
        ])
        .add_row(vec![
            Cell::new("Minimum Balance (SOL)"),
            Cell::new(format!("{:.9}", lamports_to_sol(min_balance))),
        ]);

    println!("\n{}", style("RENT EXEMPTION").green().bold());
    println!("{table}");

    Ok(())
}

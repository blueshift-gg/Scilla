use {
    crate::{
        commands::CommandExec, context::ScillaContext, error::ScillaResult,
        misc::helpers::lamports_to_sol, prompt::prompt_data, ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::Select,
    solana_nonce::{state::Data, versions::Versions},
    solana_pubkey::Pubkey,
    solana_rpc_client_api::config::{RpcLargestAccountsConfig, RpcLargestAccountsFilter},
    solana_signature::Signature,
};

/// Commands related to wallet or account management
#[derive(Debug, Clone)]
pub enum AccountCommand {
    Fetch,
    Balance,
    Transfer,
    Airdrop,
    ConfirmTransaction,
    LargestAccounts,
    NonceAccount,
    GoBack,
}

impl AccountCommand {
    pub fn description(&self) -> &'static str {
        match self {
            AccountCommand::Fetch => "Fetch Account info",
            AccountCommand::Balance => "Get Account Balance",
            AccountCommand::Transfer => "Transfer SOL",
            AccountCommand::Airdrop => "Request Airdrop",
            AccountCommand::ConfirmTransaction => "Confirm a pending transaction",
            AccountCommand::LargestAccounts => "Fetch cluster’s largest accounts",
            AccountCommand::NonceAccount => "Inspect or manage nonce accounts",
            AccountCommand::GoBack => "Go back",
        }
    }
}

impl AccountCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            AccountCommand::Fetch => {
                let pubkey: Pubkey = prompt_data("Enter Pubkey :")?;
                show_spinner(self.description(), fetch_acc_data(ctx, &pubkey)).await?;
            }
            AccountCommand::Balance => {
                let pubkey: Pubkey = prompt_data("Enter Pubkey :")?;
                show_spinner(self.description(), fetch_account_balance(ctx, &pubkey)).await?;
            }
            AccountCommand::Transfer => {
                // show_spinner(self.description(), todo!()).await?;
            }
            AccountCommand::Airdrop => {
                show_spinner(self.description(), request_sol_airdrop(ctx)).await?;
            }
            AccountCommand::ConfirmTransaction => {
                confirm_transaction(ctx).await?;
            }
            AccountCommand::LargestAccounts => {
                show_spinner(self.description(), fetch_largest_accounts(ctx)).await?;
            }
            AccountCommand::NonceAccount => {
                fetch_nonce_account(ctx).await?;
            }
            AccountCommand::GoBack => {
                return Ok(CommandExec::GoBack);
            }
        };

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
            eprintln!(
                "{} {}",
                style("Airdrop failed:").red().bold(),
                style(err).red()
            );
        }
    }

    Ok(())
}

async fn fetch_acc_data(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let acc = ctx.rpc().get_account(pubkey).await?;

    println!(
        "{}\n{}",
        style("Account info:").green().bold(),
        style(format!("{acc:#?}")).cyan()
    );

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

async fn confirm_transaction(ctx: &ScillaContext) -> anyhow::Result<()> {
    let (signature, confirmed) = loop {
        let signature: Signature = prompt_data("Enter transaction signature:")?;

        match show_spinner("Confirming transaction", async {
            ctx.rpc()
                .confirm_transaction(&signature)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        })
        .await
        {
            Ok(confirmed) => break (signature, confirmed),
            Err(e) => {
                println!(
                    "\n{}\n",
                    style(format!("Error: Failed to confirm transaction - {}", e))
                        .red()
                        .bold()
                );
                continue;
            }
        }
    };

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
    println!("{}", table);

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
            Cell::new(format!("{:.2}", balance_sol)),
        ]);
    }

    println!("\n{}", style("LARGEST ACCOUNTS").green().bold());
    println!("{}", table);

    Ok(())
}

async fn fetch_nonce_account(ctx: &ScillaContext) -> anyhow::Result<()> {
    let (pubkey, account, data) = loop {
        let pubkey: Pubkey = prompt_data("Enter nonce account pubkey:")?;

        let account = match show_spinner("Fetching nonce account", async {
            ctx.rpc()
                .get_account(&pubkey)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        })
        .await
        {
            Ok(acc) => acc,
            Err(_e) => {
                println!(
                    "\n{}\n",
                    style(format!("Error: Account not found - {}", pubkey))
                        .red()
                        .bold()
                );
                continue;
            }
        };

        let nonce_data: Option<Data> = bincode::deserialize::<Versions>(&account.data)
            .ok()
            .and_then(|versions| match versions.state() {
                solana_nonce::state::State::Initialized(data) => Some(data.clone()),
                _ => None,
            });

        // Check if it's a nonce account, retry if not
        match nonce_data {
            Some(d) => break (pubkey, account, d),
            None => {
                println!(
                    "\n{}\n",
                    style("Error: This account is not an initialized nonce account")
                        .red()
                        .bold()
                );
                continue;
            }
        }
    };

    let min_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(80)
        .await
        .unwrap_or(0);

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
            Cell::new("Minimum Balance Required"),
            Cell::new(format!("{:.8} SOL", lamports_to_sol(min_balance))),
        ])
        .add_row(vec![
            Cell::new("Nonce blockhash"),
            Cell::new(data.blockhash().to_string()),
        ])
        .add_row(vec![
            Cell::new("Fee"),
            Cell::new(format!(
                "{} lamports per signature",
                data.get_lamports_per_signature()
            )),
        ])
        .add_row(vec![
            Cell::new("Authority"),
            Cell::new(data.authority.to_string()),
        ]);

    println!("\n{}", style("NONCE ACCOUNT INFO").green().bold());
    println!("{}", table);

    Ok(())
}

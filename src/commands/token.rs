use {
    crate::{
        commands::CommandExec,
        constants::SPL_TOKEN_PROGRAM_ID,
        context::ScillaContext,
        error::ScillaResult,
        prompt::prompt_data,
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_account_decoder::UiAccountData,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::request::TokenAccountsFilter,
    std::fmt,
};

/// SPL Token commands listed here
#[derive(Debug, Clone)]
pub enum TokenCommand {
    ListTokenAccounts,
    TokenBalance,
    MintInfo,
    GoBack,
}


impl TokenCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            TokenCommand::ListTokenAccounts => "Fetching token accounts…",
            TokenCommand::TokenBalance => "Fetching token balance…",
            TokenCommand::MintInfo => "Fetching mint info…",
            TokenCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for TokenCommand { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cmd = match self {
            TokenCommand::ListTokenAccounts => "List token accounts",
            TokenCommand::TokenBalance => "Token balance",
            TokenCommand::MintInfo => "Mint info",
            TokenCommand::GoBack => "Go back",
        };
        write!(f, "{cmd}")
    }
}

impl TokenCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            TokenCommand::ListTokenAccounts => {
                show_spinner(self.spinner_msg(), list_token_accounts(ctx)).await?;
            }
            TokenCommand::TokenBalance => {
                let token_account: Pubkey = prompt_data("Enter token account pubkey:")?;
                show_spinner(self.spinner_msg(), fetch_token_balance(ctx, &token_account)).await?;
            }
            TokenCommand::MintInfo => {
                let mint: Pubkey = prompt_data("Enter mint address:")?;
                show_spinner(self.spinner_msg(), fetch_mint_info(ctx, &mint)).await?;
            }
            TokenCommand::GoBack => return Ok(CommandExec::GoBack),
        }
        Ok(CommandExec::Process(()))
    }
}

// listing token accounts of wallet
async fn list_token_accounts(ctx: &ScillaContext) -> anyhow::Result<()> {
    // Query the old spl token program
    let accounts_v1 = ctx
        .rpc()
        .get_token_accounts_by_owner(
            ctx.pubkey(),
            TokenAccountsFilter::ProgramId(SPL_TOKEN_PROGRAM_ID.parse().unwrap()),
        )
        .await
        .unwrap_or_default();

    // Query Token-2022 program
    let accounts_v2 = ctx
        .rpc()
        .get_token_accounts_by_owner(
            ctx.pubkey(),
            TokenAccountsFilter::ProgramId(spl_token_2022::id()),
        )
        .await
        .unwrap_or_default();

    // Combine both
    let all_accounts: Vec<_> = accounts_v1.into_iter().chain(accounts_v2).collect();

    if all_accounts.is_empty() {
        println!("{}", style("No token accounts found.").yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Token Account").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Mint").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Balance").add_attribute(comfy_table::Attribute::Bold),
    ]);

    for acc in &all_accounts {
        // Parse the account data which is returned as JSON
        if let UiAccountData::Json(parsed) = &acc.account.data {
            if let Some(info) = parsed.parsed.get("info") {
                let mint = info.get("mint")
                    .and_then(|m: &serde_json::Value| m.as_str())
                    .unwrap_or("—");
                let balance = info.get("tokenAmount")
                    .and_then(|t: &serde_json::Value| t.get("uiAmountString"))
                    .and_then(|b: &serde_json::Value| b.as_str())
                    .unwrap_or("—");
                
                table.add_row(vec![
                    Cell::new(&acc.pubkey),
                    Cell::new(mint),
                    Cell::new(balance),
                ]);
            }
        }
    }

    println!("\n{}", style("TOKEN ACCOUNTS").green().bold());
    println!("{table}");
    Ok(())
}


async fn fetch_token_balance(ctx: &ScillaContext, token_account: &Pubkey) -> anyhow::Result<()> {
    let balance = ctx.rpc().get_token_account_balance(token_account).await?;
    println!(
        "\n{}\n{} {} (raw: {})",
        style("Token Balance").green().bold(),
        style(&balance.ui_amount_string).cyan(),
        style("tokens").dim(),
        balance.amount
    );
    Ok(())
}

async fn fetch_mint_info(ctx: &ScillaContext, mint: &Pubkey) -> anyhow::Result<()> {
    let account = ctx.rpc().get_account(mint).await?;

    // Deserialize as spl_token_2022::state::Mint
    let mint_data = spl_token_2022::extension::StateWithExtensionsOwned::<
        spl_token_2022::state::Mint,
    >::unpack(account.data)?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec!["Decimals", &mint_data.base.decimals.to_string()])
        .add_row(vec!["Supply", &mint_data.base.supply.to_string()])
        .add_row(vec![
            "Mint Authority",
            &mint_data.base.mint_authority
                .map(|p| p.to_string())
                .unwrap_or_else(|| "Disabled".into()),
        ])
        .add_row(vec![
            "Freeze Authority",
            &mint_data.base.freeze_authority
                .map(|p| p.to_string())
                .unwrap_or_else(|| "Disabled".into()),
        ]);

    println!("\n{}", style("MINT INFO").green().bold());
    println!("{table}");
    Ok(())
}


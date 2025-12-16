use {
    crate::{
        commands::CommandExec, constants::LAMPORTS_PER_SOL, context::ScillaContext,
        error::ScillaResult, prompt::prompt_data, ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_pubkey::Pubkey,
    std::ops::Div,
};

/// Commands related to wallet or account management
#[derive(Debug, Clone)]
pub enum AccountCommand {
    FetchAccount,
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
            AccountCommand::FetchAccount => "Fetch Account",
            AccountCommand::Balance => "Check SOL balance",
            AccountCommand::Transfer => "Send SOL to another wallet",
            AccountCommand::Airdrop => "Request devnet/testnet SOL",
            AccountCommand::ConfirmTransaction => "Check if a transaction landed",
            AccountCommand::LargestAccounts => "See the biggest accounts on cluster",
            AccountCommand::NonceAccount => "Inspect or manage durable nonces",
            AccountCommand::GoBack => "Go back",
        }
    }
}

impl AccountCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            AccountCommand::FetchAccount => {
                let pubkey: Pubkey = prompt_data("Enter Pubkey:")?;
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
                // show_spinner(self.description(), todo!()).await?;
            }
            AccountCommand::LargestAccounts => {
                // show_spinner(self.description(), todo!()).await?;
            }
            AccountCommand::NonceAccount => {
                // show_spinner(self.description(), todo!()).await?;
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

    println!(
        "\n{}\n{}",
        style("ACCOUNT INFORMATION").green().bold(),
        table
    );

    Ok(())
}

async fn fetch_account_balance(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let acc = ctx.rpc().get_account(pubkey).await?;
    let acc_balance: f64 = if acc.lamports > 0 {
        (acc.lamports as f64).div(LAMPORTS_PER_SOL as f64)
    } else {
        0.0
    };

    println!(
        "{}\n{}",
        style("Account balance in SOL:").green().bold(),
        style(format!("{acc_balance:#?}")).cyan()
    );

    Ok(())
}

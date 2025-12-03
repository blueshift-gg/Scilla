use console::style;
use solana_pubkey::Pubkey;

use crate::{
    commands::CommandExec,
    context::ScillaContext,
    error::ScillaResult,
    misc::{
        helpers::{build_transfer_transaction, checked_sol_to_lamports, get_explorer_url, lamports_to_sol},
        validation::validate_transfer_params,
    },
    prompt::prompt_data,
    ui::show_spinner,
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
                show_spinner(self.description(), transfer_sol(ctx)).await?;
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
        };

        Ok(CommandExec::Process(()))
    }
}

async fn request_sol_airdrop(ctx: &ScillaContext) -> anyhow::Result<()> {
    use anyhow::Context;
    
    let amount_sol: f64 = prompt_data("Enter amount in SOL:")
        .context("Failed to parse amount. Please enter a valid number.")?;

    let lamports = checked_sol_to_lamports(amount_sol)?;
    
    let sig = ctx.rpc().request_airdrop(ctx.pubkey(), lamports).await;
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
                style(&err).red()
            );
            return Err(err.into());
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
    let acc_balance: f64 = lamports_to_sol(acc.lamports);

    println!(
        "{}\n{}",
        style("Account balance in SOL:").green().bold(),
        style(format!("{acc_balance:#?}")).cyan()
    );

    Ok(())
}

async fn simulate_and_validate_transfer(
    ctx: &ScillaContext,
    destination: &Pubkey,
    lamports: u64,
) -> anyhow::Result<(u64, u64)> {
    use anyhow::Context;

    println!("\n{}", style("━".repeat(60)).dim());
    println!("{}", style("Simulating transaction...").dim());

    let test_transaction = build_transfer_transaction(ctx, destination, lamports).await?;

    let simulation_result = ctx
        .rpc()
        .simulate_transaction(&test_transaction)
        .await
        .context("Failed to simulate transaction. Check your RPC connection.")?;

    const FALLBACK_FEE_LAMPORTS: u64 = 5_000;
    let actual_fee_lamports = simulation_result.value.fee.unwrap_or(FALLBACK_FEE_LAMPORTS);

    if let Some(err) = &simulation_result.value.err {
        return Err(anyhow::anyhow!(
            "Transaction simulation failed: {err:?}"
        ));
    }

    let current_balance = ctx
        .rpc()
        .get_balance(ctx.pubkey())
        .await
        .context("Failed to fetch current balance. Check your RPC connection.")?;

    Ok((current_balance, actual_fee_lamports))
}


async fn execute_transfer(
    ctx: &ScillaContext,
    destination: &Pubkey,
    lamports: u64,
    amount_sol: f64,
    actual_fee_lamports: u64,
) -> anyhow::Result<()> {
    use anyhow::Context;

    println!("{}", style("Verifying balance...").dim());
    let latest_balance = ctx
        .rpc()
        .get_balance(ctx.pubkey())
        .await
        .context("Failed to fetch current balance before sending. Check your RPC connection.")?;

    let required_lamports = lamports.saturating_add(actual_fee_lamports);
    if latest_balance < required_lamports {
        return Err(anyhow::anyhow!(
            "Balance insufficient before send. Current balance: {} SOL, Required at least: {} SOL. \
Balance may have changed since simulation.",
            lamports_to_sol(latest_balance),
            lamports_to_sol(required_lamports),
        ));
    }

    println!("{}", style("Sending transaction...").dim());

    let transaction = build_transfer_transaction(ctx, destination, lamports).await?;

    let signature = ctx
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await
        .with_context(|| {
            format!(
                "Transaction failed. Transfer of {amount_sol} SOL from {} to {} could not be completed. Please check your balance and try again.",
                ctx.pubkey(), destination
            )
        })?;
    
    let new_balance = ctx
        .rpc()
        .get_balance(ctx.pubkey())
        .await
        .context("Transfer succeeded but failed to fetch updated balance.")?;

    println!("\n{}", style("Transfer successful!").green().bold());
    println!(
        "{:<15} {}",
        style("Signature:").bold(),
        style(&signature).cyan()
    );

    let explorer_url = get_explorer_url(&signature, ctx);
    println!(
        "{:<15} {}",
        style("Explorer:").bold(),
        style(&explorer_url).cyan().underlined()
    );

    println!(
        "{:<15} {} SOL",
        style("New Balance:").bold(),
        style(format!("{:.9}", lamports_to_sol(new_balance))).green()
    );

    Ok(())
}


async fn transfer_sol(ctx: &ScillaContext) -> anyhow::Result<()> {
    use anyhow::Context;
    use inquire::Confirm;

    let destination: Pubkey = prompt_data("Enter destination address:")
        .context("Failed to parse destination address. Please enter a valid Solana pubkey.")?;

    let amount_sol: f64 = prompt_data("Enter amount in SOL:")
        .context("Failed to parse amount. Please enter a valid number.")?;

    let lamports = validate_transfer_params(ctx.pubkey(), &destination, amount_sol)?;

    let (current_balance, actual_fee_lamports) =
        simulate_and_validate_transfer(ctx, &destination, lamports).await?;

    let fee_sol = lamports_to_sol(actual_fee_lamports);
    let current_balance_sol = lamports_to_sol(current_balance);
    let estimated_balance_after =
        current_balance.saturating_sub(lamports).saturating_sub(actual_fee_lamports);
    let estimated_balance_after_sol = lamports_to_sol(estimated_balance_after);

    println!("\n{}", style("Transfer summary").bold().cyan());
    println!(
        "{:<15} {}",
        style("From:").bold(),
        style(ctx.pubkey()).cyan()
    );
    println!(
        "{:<15} {}",
        style("To:").bold(),
        style(destination).cyan()
    );
    println!(
        "{:<15} {} SOL",
        style("Amount:").bold(),
        style(format!("{amount_sol:.9}")).green()
    );
    println!(
        "{:<15} {} SOL",
        style("Current balance:").bold(),
        style(format!("{current_balance_sol:.9}")).yellow()
    );
    println!(
        "{:<15} {} SOL",
        style("Estimated fee:").bold(),
        style(format!("{fee_sol:.9}")).green()
    );
    println!(
        "{:<15} {} SOL",
        style("Estimated after:").bold(),
        style(format!("{estimated_balance_after_sol:.9}")).cyan()
    );

    let confirmed = Confirm::new("Send transaction?")
        .with_default(false)
        .prompt()?;

    if !confirmed {
        println!("{}", style("Transfer cancelled").yellow());
        return Ok(());
    }

    execute_transfer(ctx, &destination, lamports, amount_sol, actual_fee_lamports).await?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use crate::misc::validation::validate_transfer_params;
    use std::str::FromStr;
    use solana_pubkey::Pubkey;

    fn test_sender() -> Pubkey {
        Pubkey::from_str("11111111111111111111111111111112").unwrap()
    }

    fn test_recipient() -> Pubkey {
        Pubkey::from_str("11111111111111111111111111111113").unwrap()
    }

    #[test]
    fn test_validate_transfer_params_valid_transfer() {
        let sender = test_sender();
        let recipient = test_recipient();
        let amount = 1.5;

        let result = validate_transfer_params(&sender, &recipient, amount);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1_500_000_000);
    }

    #[test]
    fn test_validate_transfer_params_self_transfer_rejected() {
        let sender = test_sender();

        let result = validate_transfer_params(&sender, &sender, 1.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot send to self"));
    }

    #[test]
    fn test_validate_transfer_params_negative_amount() {
        let sender = test_sender();
        let recipient = test_recipient();

        let result = validate_transfer_params(&sender, &recipient, -1.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Amount must be positive"));
    }

    #[test]
    fn test_validate_transfer_params_zero_amount() {
        let sender = test_sender();
        let recipient = test_recipient();

        let result = validate_transfer_params(&sender, &recipient, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Amount must be positive"));
    }

    #[test]
    fn test_validate_transfer_params_too_small_amount() {
        let sender = test_sender();
        let recipient = test_recipient();
        let tiny_amount = 0.0000000001; 

        let result = validate_transfer_params(&sender, &recipient, tiny_amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Amount too small"));
    }

    #[test]
    fn test_validate_transfer_params_minimum_valid_amount() {
        let sender = test_sender();
        let recipient = test_recipient();
        let min_amount = 0.000000001; 

        let result = validate_transfer_params(&sender, &recipient, min_amount);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_validate_transfer_params_max_amount_too_large() {
        let sender = test_sender();
        let recipient = test_recipient();
        let huge_amount = 1e20; 

        let result = validate_transfer_params(&sender, &recipient, huge_amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Amount too large"));
    }

    #[test]
    fn test_validate_transfer_params_typical_amounts() {
        let sender = test_sender();
        let recipient = test_recipient();

        let test_cases = vec![
            (0.1, 100_000_000),
            (0.5, 500_000_000),
            (1.0, 1_000_000_000),
            (10.0, 10_000_000_000),
            (100.0, 100_000_000_000),
        ];

        for (amount_sol, expected_lamports) in test_cases {
            let result = validate_transfer_params(&sender, &recipient, amount_sol);
            assert!(result.is_ok(), "Failed for amount {}", amount_sol);
            assert_eq!(result.unwrap(), expected_lamports, "Wrong conversion for {}", amount_sol);
        }
    }

}


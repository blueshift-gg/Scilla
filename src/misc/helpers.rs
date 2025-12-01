use crate::context::ScillaContext;
use anyhow::{Context, Result};
use console::style;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction;
use solana_transaction::Transaction;

use crate::misc::conversion::lamports_to_sol;


/// # Errors
/// Returns an error if:
/// - RPC connection fails
/// - Failed to get recent blockhash
pub async fn build_transfer_transaction(
    ctx: &ScillaContext,
    destination: &Pubkey,
    lamports: u64,
) -> Result<Transaction> {
    let transfer_instruction = instruction::transfer(ctx.pubkey(), destination, lamports);
    let recent_blockhash = ctx
        .rpc()
        .get_latest_blockhash()
        .await
        .context("Failed to get recent blockhash. Check your RPC connection.")?;

    let message = Message::new(std::slice::from_ref(&transfer_instruction), Some(ctx.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(&[ctx.keypair()], recent_blockhash);

    Ok(transaction)
}

pub fn display_transfer_confirmation(
    ctx: &ScillaContext,
    destination: &Pubkey,
    amount_sol: f64,
    lamports: u64,
    current_balance: u64,
    actual_fee_lamports: u64,
) {
    let actual_fee_sol = lamports_to_sol(actual_fee_lamports);

    println!("\n{}", style("━".repeat(60)).dim());
    println!("{}", style("Transfer Confirmation").bold().cyan());
    println!("{}", style("━".repeat(60)).dim());
    println!(
        "{:<12} {}",
        style("From:").bold(),
        style(ctx.pubkey()).cyan()
    );
    println!(
        "{:<12} {}",
        style("To:").bold(),
        style(destination).cyan()
    );
    println!(
        "{:<12} {} SOL ({} lamports)",
        style("Amount:").bold(),
        style(amount_sol).green(),
        style(lamports).dim()
    );
    println!(
        "{:<12} {} SOL",
        style("Current Balance:").bold(),
        style(lamports_to_sol(current_balance)).yellow()
    );
    println!(
        "{:<12} {} SOL",
        style("Fee (actual):").bold(),
        style(format!("{actual_fee_sol:.9}")).green()
    );
    println!(
        "{:<12} {} SOL",
        style("Balance after:").bold(),
        style(lamports_to_sol(current_balance.saturating_sub(lamports).saturating_sub(actual_fee_lamports))).cyan()
    );
    println!("{}\n", style("━".repeat(60)).dim());
}

pub fn get_network_cluster(rpc_url: &str) -> &str {
    let hostname = if let Some(start) = rpc_url.find("://") {
        let after_scheme = &rpc_url[start + 3..];
        if let Some(end) = after_scheme.find('/') {
            &after_scheme[..end]
        } else {
            after_scheme
        }
    } else {
        rpc_url
    };

    let host_only = if let Some(colon_pos) = hostname.find(':') {
        &hostname[..colon_pos]
    } else {
        hostname
    };

    if host_only.contains("mainnet-beta") {
        ""
    } else if host_only.contains("devnet") {
        "?cluster=devnet"
    } else if host_only.contains("testnet") {
        "?cluster=testnet"
    } else {
        "?cluster=custom"
    }
}

pub fn get_explorer_url(signature: impl std::fmt::Display, ctx: &ScillaContext) -> String {
    let rpc_url = ctx.rpc_url();
    let network = get_network_cluster(rpc_url);
    format!("https://explorer.solana.com/tx/{signature}{network}")
}


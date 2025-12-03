use crate::{context::ScillaContext, config::ScillaConfig};
use anyhow::{anyhow, Context, Result};
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction;
use solana_transaction::Transaction;
use crate::constants::LAMPORTS_PER_SOL;

pub const MAX_LAMPORT_AMOUNT: u64 = u64::MAX;


pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

pub fn checked_sol_to_lamports(amount_sol: f64) -> Result<u64> {
    if !amount_sol.is_finite() {
        return Err(anyhow!(
            "Amount must be a finite number. You entered: {}",
            amount_sol
        ));
    }

    let lamports_f64 = amount_sol * LAMPORTS_PER_SOL as f64;

    if lamports_f64 <= 0.0 {
        return Err(anyhow!(
            "Amount must be positive. You entered: {} SOL",
            amount_sol
        ));
    }

    if lamports_f64 < 1.0 {
        return Err(anyhow!(
            "Amount too small. Must be at least 0.000000001 SOL (1 lamport). You entered: {} SOL",
            amount_sol
        ));
    }

    if lamports_f64 > MAX_LAMPORT_AMOUNT as f64 {
        let max_sol_amount = MAX_LAMPORT_AMOUNT as f64 / LAMPORTS_PER_SOL as f64;
        return Err(anyhow!(
            "Amount too large. Maximum supported: {} SOL",
            max_sol_amount
        ));
    }

    Ok(lamports_f64 as u64)
}

pub async fn build_transaction<F>(
    ctx: &ScillaContext,
    message_builder: F,
) -> Result<Transaction>
where
    F: FnOnce() -> Message,
{
    let recent_blockhash = ctx
        .rpc()
        .get_latest_blockhash()
        .await
        .context("Failed to get recent blockhash. Check your RPC connection.")?;

    let message = message_builder();
    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(&[ctx.keypair()], recent_blockhash);

    Ok(transaction)
}

pub async fn build_transfer_transaction(
    ctx: &ScillaContext,
    destination: &Pubkey,
    lamports: u64,
) -> Result<Transaction> {
    let transfer_instruction = instruction::transfer(ctx.pubkey(), destination, lamports);
    build_transaction(ctx, || {
        Message::new(std::slice::from_ref(&transfer_instruction), Some(ctx.pubkey()))
    })
    .await
}

pub fn get_explorer_url(signature: impl std::fmt::Display, ctx: &ScillaContext) -> String {
    ScillaConfig::explorer_url_for_cluster(signature, ctx.cluster())
}


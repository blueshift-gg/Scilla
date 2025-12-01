use crate::misc::conversion::sol_to_lamports;
use anyhow::Result;
use solana_pubkey::Pubkey;

pub const MAX_SOL_AMOUNT: f64 = 1_000_000_000.0; 

/// # Errors
/// Returns an error if:
/// - Amount is zero or negative
/// - Amount exceeds maximum supported value (`MAX_SOL_AMOUNT` SOL)
/// - Amount is too small (less than 1 lamport)
pub fn validate_amount(amount_sol: f64) -> Result<u64> {
    if amount_sol <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be positive. You entered: {} SOL", amount_sol));
    }

    if amount_sol > MAX_SOL_AMOUNT {
        return Err(anyhow::anyhow!(
            "Amount too large. Maximum supported: {} SOL",
            MAX_SOL_AMOUNT
        ));
    }

    let lamports = sol_to_lamports(amount_sol);

    if lamports == 0 {
        return Err(anyhow::anyhow!(
            "Amount too small. Minimum supported: 0.000000001 SOL (1 lamport)"
        ));
    }

    Ok(lamports)
}

/// Validate transfer parameters before making RPC calls
///
/// # Errors
/// Returns an error if:
/// - Destination is the same as sender (self-transfer)
/// - Amount is zero or negative
/// - Amount exceeds maximum supported value (u64::MAX lamports)
/// - Amount is too small (less than 1 lamport)
pub fn validate_transfer_params(
    sender: &Pubkey,
    destination: &Pubkey,
    amount_sol: f64,
) -> Result<u64> {
    if destination == sender {
        return Err(anyhow::anyhow!(
            "Cannot send to self. Destination must be different from sender address."
        ));
    }

    validate_amount(amount_sol)
}

pub fn validate_balance(
    balance_lamports: u64,
    transfer_lamports: u64,
    fee_lamports: u64,
) -> Result<()> {
    use crate::misc::conversion::lamports_to_sol;

    let required = transfer_lamports.saturating_add(fee_lamports);
    if balance_lamports < required {
        return Err(anyhow::anyhow!(
            "Insufficient balance. Required: {} SOL (including {} SOL fee), Available: {} SOL",
            lamports_to_sol(required),
            lamports_to_sol(fee_lamports),
            lamports_to_sol(balance_lamports),
        ));
    }
    Ok(())
}


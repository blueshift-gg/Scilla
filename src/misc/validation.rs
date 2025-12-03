use anyhow::Result;
use solana_pubkey::Pubkey;
use crate::misc::helpers::checked_sol_to_lamports;


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

    checked_sol_to_lamports(amount_sol)
}


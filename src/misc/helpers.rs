use {
    crate::{ScillaContext, constants::LAMPORTS_PER_SOL},
    anyhow::{anyhow, bail},
    solana_instruction::Instruction,
    solana_keypair::{EncodableKey, Keypair, Signature, Signer},
    solana_message::Message,
    solana_transaction::Transaction,
    std::path::Path,
};

pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL as f64) as u64
}

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

pub fn parse_sol_amount(amount_str: &str) -> anyhow::Result<u64> {
    let trimmed = amount_str.trim();
    if trimmed.is_empty() {
        Ok(0)
    } else {
        let sol: f64 = trimmed
            .parse()
            .map_err(|_| anyhow!("Invalid amount: {}", trimmed))?;
        Ok(sol_to_lamports(sol))
    }
}

pub fn parse_commission(input: &str) -> anyhow::Result<u8> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0); // default to 0%
    }
    let commission: u8 = trimmed
        .parse()
        .map_err(|_| anyhow!("Invalid commission: {}", trimmed))?;
    if commission > 100 {
        bail!("Commission must be between 0 and 100");
    }
    Ok(commission)
}

pub fn read_keypair_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Keypair> {
    let path = path.as_ref();
    Keypair::read_from_file(path)
        .map_err(|e| anyhow!("Failed to read keypair from {}: {}", path.display(), e))
}

pub async fn build_and_send_tx(
    ctx: &ScillaContext,
    instruction: &[Instruction],
    signers: &[&dyn Signer],
) -> anyhow::Result<Signature> {
    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    let message = Message::new(instruction, Some(ctx.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.try_sign(&signers.to_vec(), recent_blockhash)?;

    let signature = ctx.rpc().send_and_confirm_transaction(&tx).await?;

    Ok(signature)
}

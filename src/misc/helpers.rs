use {
    crate::{constants::LAMPORTS_PER_SOL, ScillaContext},
    anyhow::{anyhow, bail},
    solana_instruction::Instruction,
    solana_keypair::{EncodableKey, Keypair, Signature, Signer},
    solana_message::Message,
    solana_transaction::Transaction,
    std::{path::Path, str::FromStr},
};

pub fn trim_and_parse<T: FromStr>(s: &str, field_name: &str) -> anyhow::Result<Option<T>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse()
            .map(Some)
            .map_err(|_| anyhow!("Invalid {field_name}: {trimmed}. Must be a valid number"))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Commission(u8);

impl Commission {
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl FromStr for Commission {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let commission = match trim_and_parse::<u8>(s, "commission")? {
            Some(val) => val,
            None => return Ok(Commission(0)), // default to 0%
        };
        if commission > 100 {
            bail!("Commission must be between 0 and 100, got {commission}");
        }
        Ok(Commission(commission))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SolAmount(f64);

impl SolAmount {
    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn to_lamports(&self) -> u64 {
        sol_to_lamports(self.0)
    }
}

impl FromStr for SolAmount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sol = trim_and_parse::<f64>(s, "amount")?
            .ok_or_else(|| anyhow!("Amount cannot be empty. Please enter a SOL amount"))?;

        if sol <= 0.0 || !sol.is_finite() {
            bail!("Amount must be a positive finite number, got {sol}");
        }
        if sol * LAMPORTS_PER_SOL as f64 > u64::MAX as f64 {
            bail!("Amount too large: {sol} SOL would overflow");
        }
        Ok(SolAmount(sol))
    }
}

pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL as f64) as u64
}

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
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

use {
    crate::{ScillaContext, constants::LAMPORTS_PER_SOL},
    anyhow::{Context, anyhow, bail},
    base64::Engine,
    bincode::Options,
    solana_account::Account,
    solana_epoch_info::EpochInfo,
    solana_instruction::Instruction,
    solana_keypair::{EncodableKey, Keypair, Signature, Signer},
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_transaction::Transaction,
    std::{path::Path, str::FromStr},
    tokio::try_join,
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

pub async fn check_minimum_balance(
    ctx: &ScillaContext,
    payer: &Pubkey,
    required_lamports: u64,
) -> anyhow::Result<()> {
    let payer_balance = ctx.rpc().get_balance(payer).await?;

    if payer_balance < required_lamports {
        bail!(
            "Insufficient balance\nRequired: {} SOL\nAvailable: {} SOL\nShort: {} SOL",
            required_lamports as f64 / 1e9,
            payer_balance as f64 / 1e9,
            (required_lamports - payer_balance) as f64 / 1e9
        );
    }

    Ok(())
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

/// Fetches account data and current epoch info in parallel.
pub async fn fetch_account_with_epoch(
    ctx: &ScillaContext,
    pubkey: &Pubkey,
) -> anyhow::Result<(Account, EpochInfo)> {
    try_join!(
        async {
            ctx.rpc()
                .get_account(pubkey)
                .await
                .map_err(|_| anyhow!("{pubkey} account does not exist"))
        },
        async {
            ctx.rpc()
                .get_epoch_info()
                .await
                .map_err(anyhow::Error::from)
        }
    )
}

/// Generic helper to deserialize bincode data with consistent error
/// context
pub fn bincode_deserialize<T>(data: &[u8], ctx: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    bincode::deserialize::<T>(data).with_context(|| format!("Failed to deserialize {}", ctx))
}

/// Generic helper to deserialize bincode data with limit and proper error
/// context
pub fn bincode_deserialize_with_limit<T>(limit: u64, data: &[u8], ctx: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    bincode::options()
        .with_fixint_encoding()
        .with_limit(limit)
        .deserialize::<T>(data)
        .with_context(|| format!("Failed to deserialize {}", ctx))
}

pub fn decode_base64(encoded: &str) -> anyhow::Result<Vec<u8>> {
    let trimmed = encoded.trim();
    if trimmed.is_empty() {
        bail!("Encoded data cannot be empty");
    }

    base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to decode Base64: {}. Please ensure the data is valid Base64 encoded.",
                e
            )
        })
}

pub fn decode_base58(encoded: &str) -> anyhow::Result<Vec<u8>> {
    let trimmed = encoded.trim();
    if trimmed.is_empty() {
        bail!("Encoded data cannot be empty");
    }

    bs58::decode(trimmed).into_vec().map_err(|e| {
        anyhow::anyhow!(
            "Failed to decode Base58: {}. Please ensure the data is valid Base58 encoded.",
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use {
        super::*, crate::constants::MEMO_PROGRAM_ID, solana_message::VersionedMessage,
        solana_transaction::versioned::VersionedTransaction,
    };

    #[test]
    fn test_lamports_to_sol_exact_one_sol() {
        assert_eq!(lamports_to_sol(1_000_000_000), 1.0);
    }

    #[test]
    fn test_lamports_to_sol_max_u64() {
        // u64::MAX lamports should not panic or overflow
        let result = lamports_to_sol(u64::MAX);
        assert!(result > 0.0, "Should handle u64::MAX without panic");
        assert!(result < f64::INFINITY, "Should not overflow to infinity");
    }
    #[test]
    fn test_decode_base64_memo_transaction() -> anyhow::Result<()> {
        // Fixture: Real memo transaction from Solana devnet
        const EXPECTED_SIGNATURE: &str = "2Bpup7xRM9TZ83J5Pk1wfECTcLyUXxb9nr4Buuv6UmePi5WjeiX4iZCPvcVwfkHj3Yanez6BWwLyEPyWydN9S6Hm";
        const BASE64_TX: &str = "ATtaXBp3r800LbtPPC2iVkX22tKZkdkjzpaC1LOYy1SdiDmSSZXwvZTp0wl+y6fbzD7mSqs96e6g0K/YKJCqnAgBAAECuWsEsgM+Pjf2OiBR/sp5JD2IQPCSzSZb1z8en71VQy8FSlNamSkhBk0k6HFg2jh8fDW13bySu4HkH6hAQQVEjQbTKauGdNvrXHjR1ToMle1qSSO+Byroa3YXytgwv3XsAQEAC2Rldm5ldC10ZXN0";

        let decoded = decode_base64(BASE64_TX)?;
        let tx: VersionedTransaction = bincode_deserialize(&decoded, "transaction")?;

        assert_eq!(tx.signatures[0].to_string(), EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_decode_base58_memo_transaction() -> anyhow::Result<()> {
        // Fixture: Real memo transaction from Solana devnet
        const EXPECTED_SIGNATURE: &str = "2Bpup7xRM9TZ83J5Pk1wfECTcLyUXxb9nr4Buuv6UmePi5WjeiX4iZCPvcVwfkHj3Yanez6BWwLyEPyWydN9S6Hm";
        const BASE64_TX: &str = "ATtaXBp3r800LbtPPC2iVkX22tKZkdkjzpaC1LOYy1SdiDmSSZXwvZTp0wl+y6fbzD7mSqs96e6g0K/YKJCqnAgBAAECuWsEsgM+Pjf2OiBR/sp5JD2IQPCSzSZb1z8en71VQy8FSlNamSkhBk0k6HFg2jh8fDW13bySu4HkH6hAQQVEjQbTKauGdNvrXHjR1ToMle1qSSO+Byroa3YXytgwv3XsAQEAC2Rldm5ldC10ZXN0";

        // Derive Base58 from Base64
        let tx_bytes = base64::engine::general_purpose::STANDARD.decode(BASE64_TX)?;
        let base58_tx = bs58::encode(&tx_bytes).into_string();

        let decoded = decode_base58(&base58_tx)?;
        let tx: VersionedTransaction = bincode_deserialize(&decoded, "transaction")?;

        assert_eq!(tx.signatures[0].to_string(), EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_memo_transaction_contains_memo_instruction() -> anyhow::Result<()> {
        // Fixture: Real memo transaction from Solana devnet
        const BASE64_TX: &str = "ATtaXBp3r800LbtPPC2iVkX22tKZkdkjzpaC1LOYy1SdiDmSSZXwvZTp0wl+y6fbzD7mSqs96e6g0K/YKJCqnAgBAAECuWsEsgM+Pjf2OiBR/sp5JD2IQPCSzSZb1z8en71VQy8FSlNamSkhBk0k6HFg2jh8fDW13bySu4HkH6hAQQVEjQbTKauGdNvrXHjR1ToMle1qSSO+Byroa3YXytgwv3XsAQEAC2Rldm5ldC10ZXN0";

        let decoded = decode_base64(BASE64_TX)?;
        let tx: VersionedTransaction = bincode_deserialize(&decoded, "transaction")?;

        let VersionedMessage::Legacy(message) = &tx.message else {
            panic!("Expected legacy message format");
        };

        let memo_program_pubkey = Pubkey::from_str(MEMO_PROGRAM_ID)?;
        let has_memo = message
            .instructions
            .iter()
            .any(|ix| message.account_keys[ix.program_id_index as usize] == memo_program_pubkey);

        assert!(has_memo, "Transaction should contain memo instruction");

        Ok(())
    }
}

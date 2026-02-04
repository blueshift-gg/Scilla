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
    solana_transaction::{Transaction, versioned::VersionedTransaction},
    solana_transaction_status::{
        EncodedTransaction, EncodedTransactionWithStatusMeta, TransactionBinaryEncoding,
        UiTransactionEncoding,
    },
    std::{
        path::Path,
        process::{Command, Stdio},
        str::FromStr,
    },
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
    bincode::deserialize::<T>(data).with_context(|| format!("Failed to deserialize {ctx}"))
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
        .with_context(|| format!("Failed to deserialize {ctx}"))
}

pub fn decode_base64(encoded: &str) -> anyhow::Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to decode Base64: {e}. Please ensure the data is valid Base64 encoded."
            )
        })
}

pub fn decode_base58(encoded: &str) -> anyhow::Result<Vec<u8>> {
    bs58::decode(encoded).into_vec().map_err(|e| {
        anyhow::anyhow!(
            "Failed to decode Base58: {e}. Please ensure the data is valid Base58 encoded."
        )
    })
}

pub fn short_pubkey(pk: &Pubkey) -> String {
    let s = pk.to_string();
    let prefix = &s[..4];
    let suffix = &s[s.len() - 3..];
    format!("{prefix}...{suffix}")
}

pub fn has_command_version(command: &str) -> anyhow::Result<bool> {
    let status = Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| anyhow!("Failed to run {command}: {err}"))?;
    Ok(status.success())
}

pub fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn decode_and_deserialize_transaction(
    encoding: UiTransactionEncoding,
    encoded_tx: &str,
) -> anyhow::Result<VersionedTransaction> {
    let trimmed = encoded_tx.trim();

    if trimmed.is_empty() {
        bail!("Encoded transaction cannot be empty");
    }

    let tx_bytes = match encoding {
        UiTransactionEncoding::Base64 => decode_base64(trimmed)?,
        UiTransactionEncoding::Base58 => decode_base58(trimmed)?,
        UiTransactionEncoding::Json | UiTransactionEncoding::JsonParsed => {
            decode_rpc_json_transaction(trimmed)?
        }
        UiTransactionEncoding::Binary => {
            let Some(binary_encoding) = encoding.into_binary_encoding() else {
                bail!("Unsupported binary encoding");
            };
            decode_binary(trimmed, binary_encoding)?
        }
    };

    bincode_deserialize(&tx_bytes, "encoded transaction to VersionedTransaction")
}

fn decode_binary(blob: &str, encoding: TransactionBinaryEncoding) -> anyhow::Result<Vec<u8>> {
    match encoding {
        TransactionBinaryEncoding::Base64 => decode_base64(blob),
        TransactionBinaryEncoding::Base58 => decode_base58(blob),
    }
}

fn decode_rpc_json_transaction(json_str: &str) -> anyhow::Result<Vec<u8>> {
    if let Ok(wrapper) = serde_json::from_str::<EncodedTransactionWithStatusMeta>(json_str)
        && let Ok(bytes) = decode_encoded_transaction(wrapper.transaction)
    {
        return Ok(bytes);
    }

    // Fallback EncodedTransaction
    let encoded: EncodedTransaction = serde_json::from_str(json_str).map_err(|e| {
        anyhow!(
            "Failed to parse RPC encoded transaction JSON: {e}. Expected EncodedTransaction or \
             EncodedTransactionWithStatusMeta formats."
        )
    })?;

    decode_encoded_transaction(encoded)
}

fn decode_encoded_transaction(encoded: EncodedTransaction) -> anyhow::Result<Vec<u8>> {
    match encoded {
        EncodedTransaction::LegacyBinary(blob) => decode_base58(&blob),
        EncodedTransaction::Binary(blob, binary_encoding) => decode_binary(&blob, binary_encoding),
        EncodedTransaction::Json(_) | EncodedTransaction::Accounts(_) => {
            bail!("JSON-encoded transactions must include binary (base58/base64) data to simulate")
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*, crate::constants::MEMO_PROGRAM_ID, solana_message::VersionedMessage,
        solana_transaction::versioned::VersionedTransaction,
    };

    const MEMO_EXPECTED_SIGNATURE: &str =
        "2Bpup7xRM9TZ83J5Pk1wfECTcLyUXxb9nr4Buuv6UmePi5WjeiX4iZCPvcVwfkHj3Yanez6BWwLyEPyWydN9S6Hm";
    const MEMO_BASE64_TX: &str = "ATtaXBp3r800LbtPPC2iVkX22tKZkdkjzpaC1LOYy1SdiDmSSZXwvZTp0wl+y6fbzD7mSqs96e6g0K/YKJCqnAgBAAECuWsEsgM+Pjf2OiBR/sp5JD2IQPCSzSZb1z8en71VQy8FSlNamSkhBk0k6HFg2jh8fDW13bySu4HkH6hAQQVEjQbTKauGdNvrXHjR1ToMle1qSSO+Byroa3YXytgwv3XsAQEAC2Rldm5ldC10ZXN0";

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
        let decoded = decode_base64(MEMO_BASE64_TX)?;
        let tx: VersionedTransaction = bincode_deserialize(&decoded, "transaction")?;

        assert_eq!(tx.signatures[0].to_string(), MEMO_EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_decode_base58_memo_transaction() -> anyhow::Result<()> {
        // Derive Base58 from Base64
        let tx_bytes = base64::engine::general_purpose::STANDARD.decode(MEMO_BASE64_TX)?;
        let base58_tx = bs58::encode(&tx_bytes).into_string();

        let decoded = decode_base58(&base58_tx)?;
        let tx: VersionedTransaction = bincode_deserialize(&decoded, "transaction")?;

        assert_eq!(tx.signatures[0].to_string(), MEMO_EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_decode_rpc_json_wrapper_transaction() -> anyhow::Result<()> {
        let wrapper = EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Binary(
                MEMO_BASE64_TX.to_string(),
                TransactionBinaryEncoding::Base64,
            ),
            meta: None,
            version: None,
        };

        let json = serde_json::to_string(&wrapper)?;

        let tx = decode_and_deserialize_transaction(UiTransactionEncoding::Json, &json)?;

        assert_eq!(tx.signatures[0].to_string(), MEMO_EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_decode_rpc_json_transaction_binary_variant() -> anyhow::Result<()> {
        let encoded = EncodedTransaction::Binary(
            MEMO_BASE64_TX.to_string(),
            TransactionBinaryEncoding::Base64,
        );

        let json = serde_json::to_string(&encoded)?;

        let tx = decode_and_deserialize_transaction(UiTransactionEncoding::JsonParsed, &json)?;

        assert_eq!(tx.signatures[0].to_string(), MEMO_EXPECTED_SIGNATURE);

        Ok(())
    }

    #[test]
    fn test_memo_transaction_contains_memo_instruction() -> anyhow::Result<()> {
        let decoded = decode_base64(MEMO_BASE64_TX)?;
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

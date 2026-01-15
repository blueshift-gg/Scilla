pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

pub const SCILLA_CONFIG_RELATIVE_PATH: &str = ".config/scilla.toml";

pub const DEFAULT_KEYPAIR_PATH: &str = ".config/solana/id.json";

pub const ACTIVE_STAKE_EPOCH_BOUND: u64 = u64::MAX;

pub const DEVNET_RPC: &str = "https://api.devnet.solana.com";

pub const MAINNET_RPC: &str = "https://api.mainnet-beta.solana.com";

pub const TESTNET_RPC: &str = "https://api.testnet.solana.com";

pub const DEFAULT_EPOCH_LIMIT: usize = 10;

pub const STAKE_HISTORY_SYSVAR_ADDR: &str = "SysvarStakeHistory1111111111111111111111111";

pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

/// Maximum chunk size for memo data in bytes.
///
/// Solana transactions have a 1232 byte limit. After accounting for:
/// - Transaction header and signatures (~64-128 bytes)
/// - Instruction overhead (program ID, account keys, instruction data length)
/// - Memo program instruction wrapper
///
/// We use 900 bytes as a safe maximum to ensure the transaction fits
/// within limits while leaving room for other instructions if needed.
pub const CHUNK_SIZE: usize = 900;

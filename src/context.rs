use {
    crate::config::ScillaConfig,
    anyhow::anyhow,
    solana_commitment_config::CommitmentConfig,
    solana_keypair::{EncodableKey, Keypair, Signer},
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    std::path::PathBuf,
};

pub struct ScillaContext {
    rpc_client: RpcClient,
    keypair: Keypair,
    pubkey: Pubkey,
    keypair_path: PathBuf,
}

impl ScillaContext {
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    pub fn rpc(&self) -> &RpcClient {
        &self.rpc_client
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    pub fn keypair_path(&self) -> &PathBuf {
        &self.keypair_path
    }

    pub fn reload(&mut self, new_config: ScillaConfig) -> anyhow::Result<()> {
        *self = ScillaContext::try_from(new_config)?;
        Ok(())
    }
}

impl TryFrom<ScillaConfig> for ScillaContext {
    type Error = anyhow::Error;

    fn try_from(config: ScillaConfig) -> anyhow::Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url,
            CommitmentConfig {
                commitment: config.commitment_level,
            },
        );

        let keypair = Keypair::read_from_file(&config.keypair_path).map_err(|e| {
            anyhow!(
                "Failed to read keypair from {}: {}",
                config.keypair_path.display(),
                e
            )
        })?;

        let pubkey = keypair.pubkey();

        Ok(Self {
            rpc_client,
            keypair,
            pubkey,
            keypair_path: config.keypair_path,
        })
    }
}

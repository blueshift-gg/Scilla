use {
    crate::config::ScillaConfig,
    solana_commitment_config::CommitmentConfig,
    solana_keypair::{EncodableKey, Keypair, Signer},
    solana_pubkey::Pubkey,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
};

pub struct ScillaContext {
    rpc_client: RpcClient,
    keypair: Keypair,
    pubkey: Pubkey,
    cluster: Option<String>,
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

    pub fn cluster(&self) -> Option<&str> {
        self.cluster.as_deref()
    }
}

impl ScillaContext {
    pub fn from_config(config: ScillaConfig) -> anyhow::Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig {
                commitment: config.commitment_level,
            },
        );

        use anyhow::anyhow;

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
            cluster: config.cluster,
        })
    }
}

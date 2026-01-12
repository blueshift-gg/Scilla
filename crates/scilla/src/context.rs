use {
    crate::{config::ScillaConfig, misc::navigation::NavContext},
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
    navigation_context: NavContext,
}

/// Creates RPC client, keypair, pubkey from config
fn create_rpc_and_keypair(config: &ScillaConfig) -> anyhow::Result<(RpcClient, Keypair, Pubkey)> {
    let rpc_client = RpcClient::new_with_commitment(
        config.rpc_url.clone(),
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

    Ok((rpc_client, keypair, pubkey))
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
        let (rpc_client, keypair, pubkey) = create_rpc_and_keypair(&new_config)?;

        // Preserve navigation context, only update RPC/keypair
        self.rpc_client = rpc_client;
        self.keypair = keypair;
        self.pubkey = pubkey;
        self.keypair_path = new_config.keypair_path;

        Ok(())
    }

    pub fn nav(&mut self) -> &mut NavContext {
        &mut self.navigation_context
    }
}

impl TryFrom<ScillaConfig> for ScillaContext {
    type Error = anyhow::Error;

    fn try_from(config: ScillaConfig) -> anyhow::Result<Self> {
        let (rpc_client, keypair, pubkey) = create_rpc_and_keypair(&config)?;

        Ok(Self {
            rpc_client,
            keypair,
            pubkey,
            keypair_path: config.keypair_path,
            navigation_context: NavContext::default(),
        })
    }
}

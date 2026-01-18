use {
    crate::{commands::navigation::NavContext, config::ScillaConfig},
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
    pubkey: Pubkey, // Cache the pubkey to avoid repeated stack allocations
    keypair_path: PathBuf,
    navigation_context: NavContext,
}

fn create_rpc_client(config: &ScillaConfig) -> anyhow::Result<RpcClient> {
    let rpc_client = RpcClient::new_with_commitment(
        config.rpc_url.clone(),
        CommitmentConfig {
            commitment: config.commitment_level,
        },
    );

    Ok(rpc_client)
}

fn load_keypair(config: &ScillaConfig) -> anyhow::Result<Keypair> {
    Keypair::read_from_file(&config.keypair_path).map_err(|e| {
        anyhow!(
            "Failed to read keypair from {}: {}",
            config.keypair_path.display(),
            e
        )
    })
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
        let rpc_client = create_rpc_client(&new_config)?;
        let keypair = load_keypair(&new_config)?;
        let pubkey = keypair.pubkey();

        // Preserve navigation context, only update RPC/keypair
        self.rpc_client = rpc_client;
        self.keypair = keypair;
        self.pubkey = pubkey;
        self.keypair_path = new_config.keypair_path;

        Ok(())
    }

    pub fn get_nav_context_mut(&mut self) -> &mut NavContext {
        &mut self.navigation_context
    }

    pub fn get_nav_context(&self) -> &NavContext {
        &self.navigation_context
    }
}

impl TryFrom<ScillaConfig> for ScillaContext {
    type Error = anyhow::Error;

    fn try_from(config: ScillaConfig) -> anyhow::Result<Self> {
        let rpc_client = create_rpc_client(&config)?;
        let keypair = load_keypair(&config)?;

        let pubkey = keypair.pubkey();

        Ok(Self {
            rpc_client,
            keypair,
            pubkey,
            keypair_path: config.keypair_path,
            navigation_context: NavContext::new(),
        })
    }
}

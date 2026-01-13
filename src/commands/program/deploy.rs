use {
    crate::{
        commands::CommandFlow,
        constants::CHUNK_SIZE,
        context::ScillaContext,
        misc::helpers::{build_and_send_tx, read_keypair_from_path},
        prompt::{prompt_confirmation, prompt_input_data},
        ui::show_spinner,
    },
    anyhow::{bail, Context},
    async_trait::async_trait,
    console::style,
    solana_keypair::{Keypair, Signer},
    solana_loader_v3_interface::{
        instruction as loader_v3_instruction, state::UpgradeableLoaderState,
    },
    solana_message::Message,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_tpu_client_next::{leader_updater::LeaderUpdater, ClientBuilder},
    std::{
        fs::File,
        io::Read,
        net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
        path::{Path, PathBuf},
        sync::Arc,
        time::Instant,
    },
    tokio_util::sync::CancellationToken,
};

pub async fn deploy(ctx: &ScillaContext) -> CommandFlow<()> {
    let program_path: String = prompt_input_data("Enter path to program .so file:");
    let keypair_path: String = prompt_input_data("Enter program keypair path:");
    let immutable = prompt_confirmation("Make program immutable (revoke upgrade authority)?");

    if !prompt_confirmation("Deploy this program?") {
        println!("{}", style("Deployment cancelled.").yellow());
        return CommandFlow::Process(());
    }

    show_spinner(
        "Deploying program via TPU/QUIC...",
        deploy_program(ctx, &program_path, &PathBuf::from(&keypair_path), immutable),
    )
    .await;

    CommandFlow::Process(())
}

/// Simple LeaderUpdater that uses RPC to get leader schedule
struct RpcLeaderUpdater {
    tpu_addresses: Vec<SocketAddr>,
}

impl RpcLeaderUpdater {
    async fn new(rpc_client: Arc<RpcClient>) -> anyhow::Result<Self> {
        // Get cluster nodes to find TPU addresses
        let cluster_nodes = rpc_client.get_cluster_nodes().await?;
        
        // Extract TPU addresses from cluster nodes
        let tpu_addresses: Vec<SocketAddr> = cluster_nodes
            .iter()
            .filter_map(|node| node.tpu)
            .collect();

        if tpu_addresses.is_empty() {
            bail!("No TPU addresses found in cluster");
        }

        Ok(Self {
            tpu_addresses,
        })
    }
}

#[async_trait]
impl LeaderUpdater for RpcLeaderUpdater {
    fn next_leaders(&mut self, lookahead_leaders: usize) -> Vec<SocketAddr> {
        // Return the first N TPU addresses
        // In a more sophisticated implementation, this would use the leader schedule
        self.tpu_addresses
            .iter()
            .take(lookahead_leaders)
            .copied()
            .collect()
    }

    async fn stop(&mut self) {
        // No cleanup needed
    }
}

async fn deploy_program(
    ctx: &ScillaContext,
    program_path: &str,
    keypair_path: &Path,
    immutable: bool,
) -> anyhow::Result<()> {
    let start_time = Instant::now();

    // Check if program file exists
    let program_path_buf = PathBuf::from(program_path);
    if !program_path_buf.exists() {
        bail!(
            "Program file not found at '{}'\n\nPlease check:\n  1. The file path is correct\n  2. The file exists\n  3. You have read permissions\n\nCurrent working directory: {}",
            program_path,
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );
    }

    if !program_path.ends_with(".so") {
        println!(
            "{}",
            style(format!(
                "Warning: File '{}' doesn't have .so extension",
                program_path
            ))
            .yellow()
        );
    }

    let mut file = File::open(program_path).context(format!(
        "Failed to open program file at '{}'",
        program_path
    ))?;
    let mut program_data = Vec::new();
    file.read_to_end(&mut program_data)?;
    let program_len = program_data.len();

    println!(
        "{}",
        style(format!("Program size: {} bytes", program_len)).dim()
    );

    let program_keypair = read_keypair_from_path(keypair_path)?;
    let program_id = program_keypair.pubkey();

    let buffer_keypair = Keypair::new();
    let buffer_pubkey = buffer_keypair.pubkey();

    println!(
        "{}",
        style(format!("Buffer account: {}", buffer_pubkey)).dim()
    );

    let buffer_len = UpgradeableLoaderState::size_of_buffer(program_len);
    let buffer_rent = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(buffer_len)
        .await?;

    let programdata_len = UpgradeableLoaderState::size_of_programdata(program_len);
    let programdata_rent = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(programdata_len)
        .await?;

    println!(
        "{} {}\n{} {}",
        style("Buffer Rent:").dim(),
        style(format!("{:.9} SOL", buffer_rent as f64 / 1_000_000_000.0)).bold(),
        style("Program Rent:").dim(),
        style(format!("{:.9} SOL", programdata_rent as f64 / 1_000_000_000.0)).bold(),
    );

    let create_buffer_ix = loader_v3_instruction::create_buffer(
        ctx.pubkey(),
        &buffer_pubkey,
        ctx.pubkey(),
        buffer_rent,
        program_len,
    )?;

    let sig = build_and_send_tx(ctx, &create_buffer_ix, &[ctx.keypair(), &buffer_keypair]).await?;
    println!("{}", style(format!("Buffer created: {}", sig)).green());

    // Prepare write transactions
    let rpc_url = ctx.rpc().url();
    let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
    let blockhash = rpc_client.get_latest_blockhash().await?;

    let mut write_transactions = Vec::new();
    let mut write_signatures = Vec::new();
    
    for (i, chunk) in program_data.chunks(CHUNK_SIZE).enumerate() {
        let offset = (i * CHUNK_SIZE) as u32;
        let write_ix = loader_v3_instruction::write(
            &buffer_pubkey,
            ctx.pubkey(),
            offset,
            chunk.to_vec(),
        );
        let message = Message::new_with_blockhash(&[write_ix], Some(ctx.pubkey()), &blockhash);
        let mut transaction = solana_transaction::Transaction::new_unsigned(message);
        transaction.try_sign(&[ctx.keypair()], blockhash)?;
        
        // Store the signature for later confirmation
        write_signatures.push(transaction.signatures[0]);
        write_transactions.push(transaction);
    }

    println!(
        "{}",
        style(format!(
            "Writing {} chunks via QUIC...",
            write_transactions.len()
        ))
        .dim()
    );

    // For reliability, we'll use a hybrid approach:
    // - Try TPU/QUIC first for speed
    // - Fall back to RPC for any transactions that don't confirm
    
    // Create leader updater
    let leader_updater = RpcLeaderUpdater::new(rpc_client.clone()).await?;

    // Setup TPU client using tpu-client-next
    let bind_socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))?;
    bind_socket.set_nonblocking(true)?;

    let cancel_token = CancellationToken::new();
    
    let (transaction_sender, client) = ClientBuilder::new(Box::new(leader_updater))
        .bind_socket(bind_socket)
        .leader_send_fanout(2)
        .identity(ctx.keypair())
        .max_cache_size(64)
        .cancel_token(cancel_token.clone())
        .build()?;

    // Serialize transactions to wire format
    let wire_transactions: Vec<Vec<u8>> = write_transactions
        .iter()
        .map(bincode::serialize)
        .collect::<Result<Vec<_>, _>>()?;

    // Send transactions in batch via TPU
    transaction_sender
        .send_transactions_in_batch(wire_transactions)
        .await
        .context("Failed to send write transactions via TPU")?;

    println!(
        "{}",
        style("Sent via QUIC, waiting for confirmations...").dim()
    );

    // Wait for confirmations with a shorter timeout for TPU
    let mut confirmed_count = 0;
    let tpu_wait_time = 10; // 10 seconds for TPU
    let mut unconfirmed_indices = Vec::new();
    
    for _ in 0..tpu_wait_time {
        let statuses = rpc_client.get_signature_statuses(&write_signatures).await?;
        
        for (idx, status_option) in statuses.value.iter().enumerate() {
            if idx < confirmed_count {
                continue;
            }
            
            if let Some(status) = status_option
                && status.confirmation_status.is_some()
                && idx == confirmed_count
            {
                confirmed_count = idx + 1;
                if confirmed_count % 10 == 0 || confirmed_count == write_signatures.len() {
                    println!(
                        "{}",
                        style(format!("Confirmed {}/{} chunks via QUIC", confirmed_count, write_signatures.len())).dim()
                    );
                }
            }
        }
        
        if confirmed_count == write_signatures.len() {
            break;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Shutdown the TPU client
    client
        .shutdown()
        .await
        .context("Failed to shutdown TPU client")?;

    // If not all confirmed via TPU, send remaining via RPC
    if confirmed_count < write_signatures.len() {
        println!(
            "{}",
            style(format!(
                "{}/{} chunks confirmed via QUIC, sending remaining via RPC for reliability...",
                confirmed_count,
                write_signatures.len()
            ))
            .yellow()
        );
        
        for idx in confirmed_count..write_signatures.len() {
            unconfirmed_indices.push(idx);
        }
        
        // Resend unconfirmed transactions via RPC
        for idx in unconfirmed_indices {
            println!(
                "{}",
                style(format!("Sending chunk {}/{} via RPC...", idx + 1, write_signatures.len())).dim()
            );
            
            ctx.rpc()
                .send_and_confirm_transaction(&write_transactions[idx])
                .await
                .context(format!("Failed to write chunk {}/{} via RPC", idx + 1, write_transactions.len()))?;
            
            confirmed_count = idx + 1;
            println!(
                "{}",
                style(format!("Confirmed {}/{} chunks", confirmed_count, write_signatures.len())).dim()
            );
        }
    }

    println!("{}", style("All chunks written to buffer").green());

    // Deploy from buffer
    // Note: deploy_with_max_program_len is marked deprecated internally but is
    // the standard way to deploy programs. Loader V4 is not yet enabled on most
    // clusters.
    #[allow(deprecated)]
    let deploy_ix = loader_v3_instruction::deploy_with_max_program_len(
        ctx.pubkey(),
        &program_id,
        &buffer_pubkey,
        ctx.pubkey(),
        programdata_rent,
        program_len,
    )?;

    let sig = build_and_send_tx(ctx, &deploy_ix, &[ctx.keypair(), &program_keypair]).await?;

    println!(
        "\n{}\n{}\n{}",
        style("Program deployed successfully!").green().bold(),
        style(format!("Program ID: {}", program_id)).cyan(),
        style(format!("Signature: {}", sig)).dim()
    );

    if immutable {
        println!("\n{}", style("Revoking upgrade authority...").yellow());
        let set_authority_ix = loader_v3_instruction::set_upgrade_authority(
            &program_id,
            ctx.pubkey(),
            None,
        );
        let auth_sig = build_and_send_tx(ctx, &[set_authority_ix], &[ctx.keypair()]).await?;
        println!(
            "{}\n{}",
            style("Program is now immutable.").red().bold(),
            style(format!("Revocation Signature: {}", auth_sig)).dim()
        );
    }

    let duration = start_time.elapsed();
    println!(
        "{}",
        style(format!(
            "Total deployment time: {:.2}s",
            duration.as_secs_f64()
        ))
        .bold()
        .green()
    );

    Ok(())
}

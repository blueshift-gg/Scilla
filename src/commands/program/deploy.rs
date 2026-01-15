use {
    crate::{
        commands::CommandFlow,
        constants::CHUNK_SIZE,
        context::ScillaContext,
        misc::helpers::{build_and_send_tx, read_keypair_from_path},
        prompt::{prompt_confirmation, prompt_input_data},
        ui::show_spinner,
    },
    anyhow::{Context, bail},
    async_trait::async_trait,
    console::style,
    solana_keypair::{Keypair, Signer},
    solana_loader_v3_interface::{
        instruction as loader_v3_instruction, state::UpgradeableLoaderState,
    },
    solana_message::Message,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_tpu_client_next::{ClientBuilder, leader_updater::LeaderUpdater},
    std::{
        fs::File,
        io::Read,
        net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
        path::{Path, PathBuf},
        str::FromStr,
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

/// Leader updater that gets actual current leaders from the cluster
struct RpcLeaderUpdater {
    tpu_map: std::collections::HashMap<solana_pubkey::Pubkey, SocketAddr>,
}

impl RpcLeaderUpdater {
    async fn new(rpc_client: Arc<RpcClient>) -> anyhow::Result<Self> {
        // Get cluster nodes to build TPU address map
        let cluster_nodes = rpc_client.get_cluster_nodes().await?;

        let mut tpu_map = std::collections::HashMap::new();
        for node in cluster_nodes {
            // STRICT QUIC: Only use nodes that explicitly advertise a QUIC port
            if let Some(tpu) = node.tpu_quic {
                // Parse the pubkey string into Pubkey
                if let Ok(pubkey) = solana_pubkey::Pubkey::from_str(&node.pubkey) {
                    tpu_map.insert(pubkey, tpu);
                }
            }
        }

        if tpu_map.is_empty() {
            bail!("No TPU addresses found in cluster");
        }

        println!(
            "{}",
            style(format!(
                "Found {} validators with QUIC support",
                tpu_map.len()
            ))
            .dim()
        );

        Ok(Self { tpu_map })
    }
}

#[async_trait]
impl LeaderUpdater for RpcLeaderUpdater {
    fn next_leaders(&mut self, lookahead_leaders: usize) -> Vec<SocketAddr> {
        // This is called synchronously, so we can't do async RPC calls here
        // Return some TPU addresses - the actual leader discovery happens at setup
        self.tpu_map
            .values()
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

    let mut file = File::open(program_path)
        .context(format!("Failed to open program file at '{}'", program_path))?;
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
        style(format!(
            "{:.9} SOL",
            programdata_rent as f64 / 1_000_000_000.0
        ))
        .bold(),
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
        // Add priority fee (micro-lamports) to ensure delivery
        let priority_fee_ix = set_compute_unit_price(50_000); // 50,000 micro-lamports (aggressive for devnet)

        let write_ix =
            loader_v3_instruction::write(&buffer_pubkey, ctx.pubkey(), offset, chunk.to_vec());

        let message = Message::new_with_blockhash(
            &[priority_fee_ix.clone(), write_ix],
            Some(ctx.pubkey()),
            &blockhash,
        );
        let mut transaction = solana_transaction::Transaction::new_unsigned(message);
        transaction.try_sign(&[ctx.keypair()], blockhash)?;

        // Store the signature for later confirmation
        write_signatures.push(transaction.signatures[0]);
        write_transactions.push(transaction);
    }

    println!(
        "{}",
        style(format!(
            "Writing {} chunks via TPU/QUIC...",
            write_transactions.len()
        ))
        .dim()
    );

    // Create leader updater with actual leader discovery
    let leader_updater = RpcLeaderUpdater::new(rpc_client.clone()).await?;

    // Setup TPU client using tpu-client-next
    // Bind to 0.0.0.0 to allow communication with external validators
    let bind_socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
    bind_socket.set_nonblocking(true)?;

    let cancel_token = CancellationToken::new();

    let (transaction_sender, client) = ClientBuilder::new(Box::new(leader_updater))
        .bind_socket(bind_socket)
        .leader_send_fanout(4) // Increased fanout for better delivery
        .identity(ctx.keypair())
        .max_cache_size(128) // Increased cache size
        .worker_channel_size(100) // Larger channel for batches
        .cancel_token(cancel_token.clone())
        .build()?;

    // Serialize transactions to wire format
    let wire_transactions: Vec<Vec<u8>> = write_transactions
        .iter()
        .map(bincode::serialize)
        .collect::<Result<Vec<_>, _>>()?;

    // Send transactions in batch via TPU
    println!("{}", style("Sending transactions to TPU leaders...").dim());
    transaction_sender
        .send_transactions_in_batch(wire_transactions.clone())
        .await
        .context("Failed to send write transactions via TPU")?;

    println!(
        "{}",
        style("Sent via QUIC, waiting for confirmations...").dim()
    );

    // Wait for confirmations with robust retry logic
    let mut confirmed = vec![false; write_transactions.len()];
    let max_wait_seconds = 60;
    let mut confirmed_count = 0;
    let mut last_resend = Instant::now();
    let resend_interval = std::time::Duration::from_secs(2);

    for elapsed_seconds in 0..max_wait_seconds {
        // Check transaction statuses
        let statuses = rpc_client.get_signature_statuses(&write_signatures).await?;

        for (idx, status_option) in statuses.value.iter().enumerate() {
            if confirmed[idx] {
                continue;
            }

            if let Some(status) = status_option {
                if status.confirmation_status.is_some() {
                    confirmed[idx] = true;
                    confirmed_count += 1;
                    println!(
                        "{}",
                        style(format!(
                            "✓ Chunk {}/{} confirmed",
                            idx + 1,
                            write_transactions.len()
                        ))
                        .green()
                    );
                }
            }
        }

        // All confirmed?
        if confirmed_count == write_transactions.len() {
            println!(
                "{}",
                style(format!(
                    "All {} chunks confirmed!",
                    write_transactions.len()
                ))
                .green()
                .bold()
            );
            break;
        }

        // Resend unconfirmed transactions if interval passed
        if last_resend.elapsed() >= resend_interval {
            let unconfirmed_wire_txs: Vec<Vec<u8>> = wire_transactions
                .iter()
                .enumerate()
                .filter(|(i, _)| !confirmed[*i])
                .map(|(_, tx)| tx.clone())
                .collect();

            if !unconfirmed_wire_txs.is_empty() {
                // We ignore errors on re-send to avoid aborting the loop; network might be flaky
                let _ = transaction_sender
                    .send_transactions_in_batch(unconfirmed_wire_txs)
                    .await;
                last_resend = Instant::now();
            }
        }

        // Show progress occasionally
        if elapsed_seconds > 0 && elapsed_seconds % 5 == 0 {
            println!(
                "{}",
                style(format!(
                    "Waiting... {}/{} confirmed (re-sending unconfirmed...)",
                    confirmed_count,
                    write_transactions.len()
                ))
                .yellow()
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Shutdown the TPU client
    client
        .shutdown()
        .await
        .context("Failed to shutdown TPU client")?;

    // Check if all were confirmed
    if confirmed_count < write_transactions.len() {
        bail!(
            "Only {}/{} chunks confirmed via QUIC after {} seconds. This might indicate:\n\
             1. Network connectivity issues to TPU\n\
             2. Validators not processing QUIC transactions\n\
             3. Blockhash expired before transactions were processed\n\n\
             Try again or check your network connection.",
            confirmed_count,
            write_transactions.len(),
            max_wait_seconds
        );
    }

    println!(
        "{}",
        style("All chunks confirmed via TPU/QUIC").green().bold()
    );

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
        let set_authority_ix =
            loader_v3_instruction::set_upgrade_authority(&program_id, ctx.pubkey(), None);
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

fn set_compute_unit_price(micro_lamports: u64) -> solana_instruction::Instruction {
    let program_id =
        solana_pubkey::Pubkey::from_str("ComputeBudget111111111111111111111111111111").unwrap();
    let mut data = vec![3u8]; // 3 is SetComputeUnitPrice tag
    data.extend_from_slice(&micro_lamports.to_le_bytes());

    solana_instruction::Instruction {
        program_id,
        accounts: vec![],
        data,
    }
}

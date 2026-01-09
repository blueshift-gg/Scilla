use {
    crate::{
        commands::CommandFlow,
        constants::CHUNK_SIZE,
        context::ScillaContext,
        misc::helpers::{build_and_send_tx, read_keypair_from_path},
        prompt::{prompt_confirmation, prompt_input_data},
        ui::show_spinner,
    },
    anyhow::{anyhow, bail},
    console::style,
    solana_client::{
        connection_cache::ConnectionCache,
        nonblocking::tpu_client::TpuClient,
        rpc_config::RpcSendTransactionConfig,
        send_and_confirm_transactions_in_parallel::{
            send_and_confirm_transactions_in_parallel_v2, SendAndConfirmConfigV2,
        },
    },
    solana_keypair::{Keypair, Signer},
    solana_loader_v3_interface::{instruction as loader_v3_instruction, state::UpgradeableLoaderState},
    solana_message::Message,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_tpu_client::tpu_client::TpuClientConfig,
    std::{fmt, fs::File, io::Read, path::PathBuf, sync::Arc},
};

#[derive(Debug, Clone)]
pub enum ProgramCommand {
    Deploy,
    GoBack,
}

impl fmt::Display for ProgramCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            ProgramCommand::Deploy => "Deploy Program",
            ProgramCommand::GoBack => "Go Back",
        };
        write!(f, "{command}")
    }
}

impl ProgramCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ProgramCommand::Deploy => "Deploying program via TPU...",
            ProgramCommand::GoBack => "",
        }
    }

    pub async fn process_command(&self, ctx: &mut ScillaContext) -> CommandFlow<()> {
        match self {
            ProgramCommand::Deploy => {
                let program_path: String = prompt_input_data("Enter path to program .so file:");
                let keypair_path: String = prompt_input_data("Enter program keypair path:");

                if !prompt_confirmation("Deploy this program?") {
                    println!("{}", style("Deployment cancelled.").yellow());
                    return CommandFlow::Process(());
                }

                show_spinner(
                    self.spinner_msg(),
                    deploy_program(ctx, &program_path, &PathBuf::from(&keypair_path)),
                )
                .await;
            }
            ProgramCommand::GoBack => {
                return CommandFlow::GoBack;
            }
        }
        CommandFlow::Process(())
    }
}

/// Deploy program via TPU/QUIC
async fn deploy_program(
    ctx: &ScillaContext,
    program_path: &str,
    keypair_path: &std::path::Path,
) -> anyhow::Result<()> {
    // 1. Read program binary
    let mut file =
        File::open(program_path).map_err(|e| anyhow!("Failed to open program file: {}", e))?;
    let mut program_data = Vec::new();
    file.read_to_end(&mut program_data)?;
    let program_len = program_data.len();

    println!(
        "{}",
        style(format!("Program size: {} bytes", program_len)).dim()
    );

    // 2. Load program keypair
    let program_keypair = read_keypair_from_path(keypair_path)?;
    let program_id = program_keypair.pubkey();

    // 3. Generate buffer keypair
    let buffer_keypair = Keypair::new();
    let buffer_pubkey = buffer_keypair.pubkey();

    println!(
        "{}",
        style(format!("Buffer account: {}", buffer_pubkey)).dim()
    );

    // 4. Calculate rent
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

    // 5. Create buffer account
    let create_buffer_ix = loader_v3_instruction::create_buffer(
        ctx.pubkey(),
        &buffer_pubkey,
        ctx.pubkey(),
        buffer_rent,
        program_len,
    )?;

    let sig = build_and_send_tx(ctx, &create_buffer_ix, &[ctx.keypair(), &buffer_keypair]).await?;
    println!("{}", style(format!("Buffer created: {}", sig)).green());

    // 6. Create write messages for chunks
    // Need to create a new RpcClient that is owned (not borrowed)
    let rpc_url = ctx.rpc().url();
    let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
    let blockhash = rpc_client.get_latest_blockhash().await?;

    let mut write_messages = Vec::new();
    for (i, chunk) in program_data.chunks(CHUNK_SIZE).enumerate() {
        let offset = (i * CHUNK_SIZE) as u32;
        let write_ix = loader_v3_instruction::write(
            &buffer_pubkey,
            ctx.pubkey(), // authority
            offset,
            chunk.to_vec(),
        );
        let message = Message::new_with_blockhash(&[write_ix], Some(ctx.pubkey()), &blockhash);
        write_messages.push(message);
    }

    println!(
        "{}",
        style(format!(
            "Writing {} chunks via TPU...",
            write_messages.len()
        ))
        .dim()
    );

    // 7. Send write transactions via TPU/QUIC
    let connection_cache = ConnectionCache::new_quic("scilla_program_deploy", 1);

    let websocket_url = rpc_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    if let ConnectionCache::Quic(cache) = connection_cache {
        let tpu_client = TpuClient::new_with_connection_cache(
            rpc_client.clone(),
            &websocket_url,
            TpuClientConfig::default(),
            cache,
        )
        .await?;

        let signers: Vec<&dyn Signer> = vec![ctx.keypair()];

        let transaction_errors = send_and_confirm_transactions_in_parallel_v2(
            rpc_client.clone(),
            Some(tpu_client),
            &write_messages,
            &signers,
            SendAndConfirmConfigV2 {
                resign_txs_count: Some(5),
                with_spinner: true,
                rpc_send_transaction_config: RpcSendTransactionConfig::default(),
            },
        )
        .await
        .map_err(|e| anyhow!("Write transactions failed: {}", e))?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if !transaction_errors.is_empty() {
            bail!("{} write transactions failed", transaction_errors.len());
        }
    }

    println!("{}", style("Program data written to buffer").green());

    // 8. Deploy from buffer
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
        style("✓ Program deployed successfully!").green().bold(),
        style(format!("Program ID: {}", program_id)).cyan(),
        style(format!("Signature: {}", sig)).dim()
    );

    Ok(())
}
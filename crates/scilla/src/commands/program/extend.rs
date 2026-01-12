use {
    crate::{
        commands::CommandFlow,
        context::ScillaContext,
        misc::helpers::{
            bincode_deserialize, build_and_send_tx, check_minimum_balance, lamports_to_sol,
        },
        prompt::{prompt_confirmation, prompt_input_data},
        ui::show_spinner,
    },
    anyhow::{Context, bail},
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_loader_v3_interface::{instruction::extend_program, state::UpgradeableLoaderState},
    solana_pubkey::Pubkey,
    solana_sdk_ids::bpf_loader_upgradeable,
};

pub async fn process_extend(ctx: &ScillaContext) -> CommandFlow<()> {
    let program_address: Pubkey = prompt_input_data("Enter Program Address: ");
    let additional_bytes: u32 = prompt_input_data("Enter Additional Bytes: ");

    if additional_bytes == 0 {
        println!(
            "{}",
            style("Error: Additional bytes must be greater than 0").red()
        );
        return CommandFlow::Process(());
    }

    let (program_data_address, current_size, additional_rent) =
        match fetch_program_info(ctx, &program_address, additional_bytes).await {
            Ok(info) => info,
            Err(e) => {
                println!("{}", style(format!("Error: {:#}", e)).red());
                return CommandFlow::Process(());
            }
        };

    let additional_rent_sol = lamports_to_sol(additional_rent);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
            Cell::new("Value")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Program Address"),
            Cell::new(program_address.to_string()),
        ])
        .add_row(vec![
            Cell::new("Program Data Address"),
            Cell::new(program_data_address.to_string()),
        ])
        .add_row(vec![
            Cell::new("Current Data Size"),
            Cell::new(format!("{} bytes", current_size)),
        ])
        .add_row(vec![
            Cell::new("Additional Bytes"),
            Cell::new(format!("{} bytes", additional_bytes)),
        ])
        .add_row(vec![
            Cell::new("New Data Size"),
            Cell::new(format!(
                "{} bytes",
                current_size + additional_bytes as usize
            )),
        ])
        .add_row(vec![
            Cell::new("Additional Rent Required"),
            Cell::new(format!("{:.9} SOL", additional_rent_sol)),
        ])
        .add_row(vec![
            Cell::new("Payer"),
            Cell::new(ctx.pubkey().to_string()),
        ]);

    println!("\n{}", style("PROGRAM EXTEND DETAILS").green().bold());
    println!("{table}\n");

    if let Err(e) = check_minimum_balance(ctx, ctx.pubkey(), additional_rent).await {
        println!("{}", style(format!("Error: {:#}", e)).red());
        return CommandFlow::Process(());
    }

    if !prompt_confirmation("Do you want to proceed with extending the program?") {
        println!("{}", style("Program extension cancelled.").yellow());
        return CommandFlow::Process(());
    }

    show_spinner(
        "Extending program account",
        execute_extend(ctx, &program_address, additional_bytes),
    )
    .await;

    CommandFlow::Process(())
}

async fn fetch_program_info(
    ctx: &ScillaContext,
    program_address: &Pubkey,
    additional_bytes: u32,
) -> anyhow::Result<(Pubkey, usize, u64)> {
    let program_account = ctx
        .rpc()
        .get_account(program_address)
        .await
        .with_context(|| format!("Failed to fetch program account: {}", program_address))?;

    if program_account.owner != bpf_loader_upgradeable::id() {
        bail!(
            "Account {} is not owned by the BPF Upgradeable Loader",
            program_address
        );
    }

    let program_state: UpgradeableLoaderState = bincode_deserialize(
        &program_account.data,
        &format!("program account {}", program_address),
    )?;

    let program_data_address = match program_state {
        UpgradeableLoaderState::Program {
            programdata_address,
        } => programdata_address,
        _ => bail!("Account {} is not a valid program account", program_address),
    };

    let program_data_account = ctx
        .rpc()
        .get_account(&program_data_address)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch program data account: {}",
                program_data_address
            )
        })?;

    let current_size = program_data_account.data.len();
    let new_size = current_size + additional_bytes as usize;

    let required_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(new_size)
        .await
        .with_context(|| "Failed to get minimum balance for rent exemption")?;

    let additional_rent = required_balance.saturating_sub(program_data_account.lamports);

    Ok((program_data_address, current_size, additional_rent))
}

async fn execute_extend(
    ctx: &ScillaContext,
    program_address: &Pubkey,
    additional_bytes: u32,
) -> anyhow::Result<()> {
    let extend_ix = extend_program(program_address, Some(ctx.pubkey()), additional_bytes);

    let signature = build_and_send_tx(ctx, &[extend_ix], &[ctx.keypair()]).await?;

    println!(
        "\n{}\n{}",
        style("Program extended successfully!").green().bold(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

use {
    crate::{
        commands::CommandFlow,
        constants::MAX_PERMITTED_DATA_LENGTH,
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
    std::fs,
};

pub async fn process_extend(ctx: &ScillaContext) -> CommandFlow<()> {
    let program_address: Pubkey = prompt_input_data("Enter Program Address: ");
    let program_path: String = prompt_input_data("Enter Program File Path: ");

    let program_file_size = match fs::metadata(&program_path) {
        Ok(metadata) => metadata.len() as usize,
        Err(e) => {
            println!(
                "{}",
                style(format!("Error: Failed to read program file: {}", e)).red()
            );
            return CommandFlow::Process(());
        }
    };

    let (program_data_address, current_size) =
        match fetch_current_program_info(ctx, &program_address).await {
            Ok(info) => info,
            Err(e) => {
                println!("{}", style(format!("Error: {:#}", e)).red());
                return CommandFlow::Process(());
            }
        };

    let additional_bytes = if program_file_size > current_size {
        (program_file_size - current_size) as u32
    } else {
        println!(
            "{}",
            style(format!(
                "Error: Program file size ({} bytes) is not larger than current program size ({} \
                 bytes). No extension needed.",
                program_file_size, current_size
            ))
            .red()
        );
        return CommandFlow::Process(());
    };

    let (additional_rent, new_size) =
        match calculate_extension_cost(ctx, &program_data_address, current_size, additional_bytes)
            .await
        {
            Ok(cost) => cost,
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
            Cell::new("Program File Path"),
            Cell::new(program_path),
        ])
        .add_row(vec![
            Cell::new("Program File Size"),
            Cell::new(format!("{} bytes", program_file_size)),
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
            Cell::new(format!("{} bytes", new_size)),
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

    if !prompt_confirmation("Do you want to proceed with extending the program? (y/n)") {
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

async fn fetch_current_program_info(
    ctx: &ScillaContext,
    program_address: &Pubkey,
) -> anyhow::Result<(Pubkey, usize)> {
    let program_account = ctx
        .rpc()
        .get_account(program_address)
        .await
        .with_context(|| format!("Failed to fetch program account: {}", program_address))?;

    if program_account.owner != bpf_loader_upgradeable::id() {
        bail!("Account {} is not an extendable program.", program_address);
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

    Ok((program_data_address, current_size))
}

async fn calculate_extension_cost(
    ctx: &ScillaContext,
    program_data_address: &Pubkey,
    current_size: usize,
    additional_bytes: u32,
) -> anyhow::Result<(u64, usize)> {
    let new_size = current_size + additional_bytes as usize;

    if new_size > MAX_PERMITTED_DATA_LENGTH {
        bail!(
            "New account size ({} bytes) would exceed the maximum permitted size of {} bytes \
             (10MB). Current size: {} bytes, additional bytes: {} bytes",
            new_size,
            MAX_PERMITTED_DATA_LENGTH,
            current_size,
            additional_bytes
        );
    }

    let program_data_account = ctx
        .rpc()
        .get_account(program_data_address)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch program data account: {}",
                program_data_address
            )
        })?;

    let required_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(new_size)
        .await
        .with_context(|| "Failed to get minimum balance for rent exemption")?;

    let additional_rent = required_balance.saturating_sub(program_data_account.lamports);

    Ok((additional_rent, new_size))
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

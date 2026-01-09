use {
    crate::{
        commands::{Command, CommandFlow, NavigationTarget, navigation::NavigationSection},
        constants::COMPUTE_BUDGET_PROGRAM_ID,
        context::ScillaContext,
        misc::helpers::decode_and_deserialize_transaction,
        prompt::{prompt_encoding_options, prompt_input_data},
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_account_decoder::UiAccount,
    solana_rpc_client_api::config::RpcTransactionConfig,
    solana_signature::Signature,
    solana_transaction_status::{
        
        EncodedTransaction, UiInnerInstructions, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,

    },
    std::{fmt, os::unix::process},
};

#[derive(Debug, Clone, Copy)]
pub enum TransactionCommand {
    CheckConfirmation,
    FetchStatus,
    FetchTransaction,
    SendTransaction,
    SimulateTransaction,
    GoBack,
}

impl TransactionCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            Self::CheckConfirmation => "Checking transaction confirmation…",
            Self::FetchStatus => "Fetching transaction status…",
            Self::FetchTransaction => "Fetching full transaction data…",
            Self::SendTransaction => "Sending transaction…",
            Self::SimulateTransaction => "Simulating transaction…",
            Self::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for TransactionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CheckConfirmation => "Check Transaction Confirmation",
            Self::FetchStatus => "Fetch Transaction Status",
            Self::FetchTransaction => "Fetch Transaction",
            Self::SendTransaction => "Send Transaction",
            Self::SimulateTransaction => "Simulate Transaction",
            Self::GoBack => "Go back",
        })
    }
}

impl Command for TransactionCommand {
    async fn process_command(&self, ctx: &mut ScillaContext) -> anyhow::Result<CommandFlow> {
        ctx.get_nav_context_mut()
            .checked_push(NavigationSection::Transaction);
        match self {
            TransactionCommand::CheckConfirmation => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                show_spinner(self.spinner_msg(), check_confirmation(ctx, &signature)).await;
            }
            TransactionCommand::FetchStatus => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                show_spinner(
                    self.spinner_msg(),
                    fetch_transaction_status(ctx, &signature),
                )
                .await;
            }
            TransactionCommand::FetchTransaction => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                let parse_instructions = inquire::Confirm::new(
                    "Do you want to parse instructions for this transaction?",
                )
                .with_default(true)
                .prompt()
                .unwrap_or(true);
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_transaction(ctx, &signature, &parse_instructions),
                )
                .await;
            }
            TransactionCommand::SendTransaction => {
                println!(
                    "{}",
                    style("Note: Only VersionedTransaction format is supported")
                        .yellow()
                        .dim()
                );

                let encoding = prompt_encoding_options();

                let encoded_tx: String = prompt_input_data("Enter encoded transaction:");

                show_spinner(
                    self.spinner_msg(),
                    send_transaction(ctx, encoding, &encoded_tx),
                )
                .await;
            }
            TransactionCommand::SimulateTransaction => {
                println!(
                    "{}",
                    style("Note: Only VersionedTransaction format is supported")
                        .yellow()
                        .dim()
                );

                let encoding = prompt_encoding_options();

                let encoded_tx: String = prompt_input_data("Enter encoded transaction:");

                show_spinner(
                    self.spinner_msg(),
                    simulate_transaction(ctx, encoding, &encoded_tx),
                )
                .await;
            }
            TransactionCommand::GoBack => {
                return Ok(CommandFlow::NavigateTo(NavigationTarget::PreviousSection));
            }
        }

        Ok(CommandFlow::Processed)
    }
}

async fn check_confirmation(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()> {
    let confirmed = ctx.rpc().confirm_transaction(signature).await?;

    let status_styled = if confirmed {
        style("Confirmed").green()
    } else {
        style("Not Confirmed").yellow()
    };

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
        .add_row(vec![Cell::new("Signature"), Cell::new(signature)])
        .add_row(vec![Cell::new("Status"), Cell::new(status_styled)]);

    println!("\n{}", style("TRANSACTION CONFIRMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn fetch_transaction_status(
    ctx: &ScillaContext,
    signature: &Signature,
) -> anyhow::Result<()> {
    let status = ctx
        .rpc()
        .get_signature_statuses_with_history(&[*signature])
        .await?;

    let Some(Some(tx_status)) = status.value.first() else {
        anyhow::bail!("Transaction not found");
    };

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
        .add_row(vec![Cell::new("Signature"), Cell::new(signature)])
        .add_row(vec![Cell::new("Slot"), Cell::new(tx_status.slot)]);

    if let Some(confirmations) = tx_status.confirmations {
        table.add_row(vec![Cell::new("Confirmations"), Cell::new(confirmations)]);
    } else {
        table.add_row(vec![
            Cell::new("Confirmations"),
            Cell::new(style("Finalized").green()),
        ]);
    }

    if let Some(confirmation_status) = &tx_status.confirmation_status {
        table.add_row(vec![
            Cell::new("Confirmation Status"),
            Cell::new(match confirmation_status {
                solana_transaction_status::TransactionConfirmationStatus::Processed => {
                    style("Processed").yellow().to_string()
                }
                solana_transaction_status::TransactionConfirmationStatus::Confirmed => {
                    style("Confirmed").cyan().to_string()
                }
                solana_transaction_status::TransactionConfirmationStatus::Finalized => {
                    style("Finalized").green().to_string()
                }
            }),
        ]);
    }

    table.add_row(vec![
        Cell::new("Status"),
        Cell::new(if tx_status.err.is_none() {
            style("Success").green().to_string()
        } else {
            style(format!("Error: {:?}", tx_status.err))
                .red()
                .to_string()
        }),
    ]);

    println!("\n{}", style("TRANSACTION STATUS").green().bold());
    println!("{table}");

    Ok(())
}

async fn process_fetch_transaction(
    ctx: &ScillaContext,
    signature: &Signature,
    parse_instructions: &bool,
) -> anyhow::Result<()> {
    let tx = ctx
        .rpc()
        .get_transaction_with_config(
            signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(ctx.rpc().commitment()),
                max_supported_transaction_version: Some(0),
            },
        )
        .await?;

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
        .add_row(vec![Cell::new("Signature"), Cell::new(signature)])
        .add_row(vec![Cell::new("Slot"), Cell::new(tx.slot)]);

    if let Some(block_time) = tx.block_time {
        table.add_row(vec![Cell::new("Block Time"), Cell::new(block_time)]);
    }

    if let Some(meta) = &tx.transaction.meta {
        table.add_row(vec![Cell::new("Fee (lamports)"), Cell::new(meta.fee)]);
        table.add_row(vec![
            Cell::new("Status"),
            Cell::new(if meta.err.is_none() {
                style("Success").green().to_string()
            } else {
                style(format!("Error: {:?}", meta.err)).red().to_string()
            }),
        ]);
    }

    println!("\n{}", style("TRANSACTION DETAILS").green().bold());
    println!("{table}");

    let EncodedTransaction::Json(ui_tx) = &tx.transaction.transaction else {
        anyhow::bail!("Transaction encoding is not JSON");
    };

    match &ui_tx.message {
        UiMessage::Parsed(parsed_msg) => {
            println!("\n{}", style("TRANSACTION MESSAGE").cyan().bold());

            let mut msg_table = Table::new();
            msg_table
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
                    Cell::new("Account Keys"),
                    Cell::new(parsed_msg.account_keys.len()),
                ])
                .add_row(vec![
                    Cell::new("Recent Blockhash"),
                    Cell::new(&parsed_msg.recent_blockhash),
                ]);

            println!("{msg_table}");

            if !parsed_msg.account_keys.is_empty() {
                println!("\n{}", style("ACCOUNT KEYS").cyan().bold());
                let mut accounts_table = Table::new();
                accounts_table.load_preset(UTF8_FULL).set_header(vec![
                    Cell::new("Index").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Pubkey").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Signer").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Writable").add_attribute(comfy_table::Attribute::Bold),
                ]);

                for (idx, account) in parsed_msg.account_keys.iter().enumerate() {
                    accounts_table.add_row(vec![
                        Cell::new(idx),
                        Cell::new(&account.pubkey),
                        Cell::new(if account.signer { "✓" } else { "" }),
                        Cell::new(if account.writable { "✓" } else { "" }),
                    ]);
                }
                println!("{accounts_table}");
            }

            if *parse_instructions {
                process_parse_instructions(parsed_msg).await?;
            }
        }
        UiMessage::Raw(raw_msg) => {
            println!("\n{}", style("TRANSACTION MESSAGE (Raw)").cyan().bold());

            let mut msg_table = Table::new();
            msg_table
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
                    Cell::new("Account Keys"),
                    Cell::new(raw_msg.account_keys.len()),
                ])
                .add_row(vec![
                    Cell::new("Recent Blockhash"),
                    Cell::new(&raw_msg.recent_blockhash),
                ]);

            println!("{msg_table}");

            if !raw_msg.account_keys.is_empty() {
                println!("\n{}", style("ACCOUNT KEYS").cyan().bold());
                for (idx, key) in raw_msg.account_keys.iter().enumerate() {
                    println!("  {idx}. {key}");
                }
            }
        }
    }

    Ok(())
}

async fn send_transaction(
    ctx: &ScillaContext,
    encoding: UiTransactionEncoding,
    encoded_tx: &str,
) -> anyhow::Result<()> {
    let tx = decode_and_deserialize_transaction(encoding, encoded_tx)?;

    let signature = ctx.rpc().send_transaction(&tx).await?;

    println!(
        "{} {}",
        style("Transaction sent successfully!").green().bold(),
        style(signature).cyan()
    );

    Ok(())
}

async fn simulate_transaction(
    ctx: &ScillaContext,
    encoding: UiTransactionEncoding,
    encoded_tx: &str,
) -> anyhow::Result<()> {
    let tx = decode_and_deserialize_transaction(encoding, encoded_tx)?;

    let response = ctx.rpc().simulate_transaction(&tx).await?;

    let value = response.value;

    println!("\n{}", style("SIMULATION RESULT").green().bold());

    let mut summary = Table::new();
    summary
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
            Cell::new("Status"),
            Cell::new(match &value.err {
                None => style("Success").green().to_string(),
                Some(err) => style(format!("{err:?}")).red().to_string(),
            }),
        ])
        .add_row(vec![
            Cell::new("Units Consumed"),
            Cell::new(value.units_consumed.unwrap_or_default()),
        ])
        .add_row(vec![
            Cell::new("Fee (lamports)"),
            Cell::new(value.fee.unwrap_or(0)),
        ])
        .add_row(vec![
            Cell::new("Replacement Blockhash"),
            Cell::new(
                value
                    .replacement_blockhash
                    .as_ref()
                    .map(|b| b.blockhash.clone())
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ])
        .add_row(vec![
            Cell::new("Loaded Data Size"),
            Cell::new(value.loaded_accounts_data_size.unwrap_or_default()),
        ]);

    println!("{summary}");

    if let Some(logs) = value.logs
        && !logs.is_empty()
    {
        println!("\n{}", style("LOGS").cyan().bold());
        for log in logs {
            println!("  • {log}");
        }
    }

    if let Some(return_data) = value.return_data {
        println!("\n{}", style("RETURN DATA").cyan().bold());
        println!("  Program: {}", return_data.program_id);
        println!("  Encoding: {:?}", return_data.data.1);
        println!("  Data: {}", return_data.data.0);
    }

    if let Some(inner) = value.inner_instructions
        && !inner.is_empty()
    {
        println!("\n{}", style("INNER INSTRUCTIONS").cyan().bold());
        for UiInnerInstructions {
            index,
            instructions,
        } in inner
        {
            println!("  At instruction {index}:");
            for (idx, ix) in instructions.into_iter().enumerate() {
                println!("    {idx}. {ix:?}");
            }
        }
    }

    if let (Some(pre), Some(post)) = (value.pre_balances, value.post_balances) {
        println!("\n{}", style("BALANCES (lamports)").cyan().bold());
        let mut bal_table = Table::new();
        bal_table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Account").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Pre").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Post").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Delta").add_attribute(comfy_table::Attribute::Bold),
        ]);
        let mut display_idx = 1;
        let mut has_rows = false;
        for (pre_amt, post_amt) in pre.into_iter().zip(post.into_iter()) {
            if pre_amt == 1 && post_amt == 1 {
                continue;
            }
            let delta: i128 = post_amt as i128 - pre_amt as i128;
            bal_table.add_row(vec![
                Cell::new(display_idx),
                Cell::new(pre_amt),
                Cell::new(post_amt),
                Cell::new(if delta > 0 {
                    format!("+{delta}")
                } else {
                    format!("{delta}")
                }),
            ]);
            display_idx += 1;
            has_rows = true;
        }
        if has_rows {
            println!("{bal_table}");
        }
    }

    if let Some(accounts) = value.accounts
        && !accounts.is_empty()
    {
        println!("\n{}", style("ACCOUNTS").cyan().bold());
        let mut accounts_table = Table::new();
        accounts_table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Index").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Pubkey").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Lamports").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Owner").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Executable").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Rent Epoch").add_attribute(comfy_table::Attribute::Bold),
        ]);

        for (idx, account_opt) in accounts.into_iter().enumerate() {
            if let Some(UiAccount {
                lamports,
                owner,
                executable,
                rent_epoch,
                ..
            }) = account_opt
            {
                accounts_table.add_row(vec![
                    Cell::new(idx),
                    Cell::new("-"),
                    Cell::new(lamports),
                    Cell::new(owner),
                    Cell::new(if executable { "✓" } else { "" }),
                    Cell::new(rent_epoch),
                ]);
            }
        }

        println!("{accounts_table}");
    }

    if let (Some(pre_tokens), Some(post_tokens)) =
        (value.pre_token_balances, value.post_token_balances)
    {
        println!("\n{}", style("TOKEN BALANCES").cyan().bold());
        let mut tok_table = Table::new();
        tok_table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Index").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Mint").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Owner").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Pre").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Post").add_attribute(comfy_table::Attribute::Bold),
        ]);

        for (idx, (pre, post)) in pre_tokens
            .into_iter()
            .zip(post_tokens.into_iter())
            .enumerate()
        {
            tok_table.add_row(vec![
                Cell::new(idx),
                Cell::new(pre.mint),
                Cell::new(pre.owner.unwrap_or_else(|| "-".to_string())),
                Cell::new(pre.ui_token_amount.ui_amount_string),
                Cell::new(post.ui_token_amount.ui_amount_string),
            ]);
        }

        println!("{tok_table}");
    }

    if let Some(loaded_addresses) = value.loaded_addresses {
        println!("\n{}", style("LOADED ADDRESSES").cyan().bold());
        let mut addr_table = Table::new();
        addr_table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Writable").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Readonly").add_attribute(comfy_table::Attribute::Bold),
        ]);
        let writable = if loaded_addresses.writable.is_empty() {
            "-".to_string()
        } else {
            loaded_addresses.writable.join(", ")
        };
        let readonly = if loaded_addresses.readonly.is_empty() {
            "-".to_string()
        } else {
            loaded_addresses.readonly.join(", ")
        };
        addr_table.add_row(vec![Cell::new(writable), Cell::new(readonly)]);
        println!("{addr_table}");
    }

    Ok(())
}

async fn process_parse_instructions(
    parsed_msg: &solana_transaction_status::UiParsedMessage,
) -> anyhow::Result<()> {
    if parsed_msg.instructions.is_empty() {
        println!("{}", style("No instructions found in transaction").yellow());
        return Ok(());
    }

    println!("\n{}", style("PARSED INSTRUCTIONS").green().bold());

    // Display each instruction
    for (idx, ui_instruction) in parsed_msg.instructions.iter().enumerate() {
        display_instruction(idx + 1, ui_instruction)?;
    }

    Ok(())
}

fn display_instruction(idx: usize, ui_instruction: &UiInstruction) -> anyhow::Result<()> {
    println!("{}", style(format!("Instruction #{}", idx)).cyan().bold());

    match ui_instruction {
        UiInstruction::Parsed(ui_parsed_ix) => match ui_parsed_ix {
            UiParsedInstruction::Parsed(parsed_ix) => display_parsed_instruction(parsed_ix),
            UiParsedInstruction::PartiallyDecoded(partial_ix) => {
                display_partially_decoded_instruction(partial_ix)
            }
        },
        UiInstruction::Compiled(compiled_ix) => display_compiled_instruction(compiled_ix),
    }
}

fn display_parsed_instruction(
    parsed_ix: &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    use serde_json::Value;

    let Value::Object(parsed_map) = &parsed_ix.parsed else {
        return display_generic_parsed(parsed_ix);
    };

    let ix_type = parsed_map
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let info = parsed_map.get("info");

    // Match on program name
    match parsed_ix.program.as_str() {
        "system" => display_system_instruction(ix_type, info),
        "spl-token" | "spl-token-2022" => display_token_instruction(ix_type, info),
        "spl-associated-token-account" => display_ata_instruction(ix_type, info),
        "spl-memo" => display_memo_instruction(info),
        _ => display_generic_parsed(parsed_ix),
    }
}

fn display_system_instruction(
    ix_type: &str,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new("System Program")])
        .add_row(vec![Cell::new("Type"), Cell::new(ix_type)]);

    if let Some(Value::Object(info_map)) = info {
        match ix_type {
            "transfer" => {
                if let (Some(source), Some(destination), Some(lamports)) = (
                    info_map.get("source").and_then(|v| v.as_str()),
                    info_map.get("destination").and_then(|v| v.as_str()),
                    info_map.get("lamports").and_then(|v| v.as_u64()),
                ) {
                    table
                        .add_row(vec![Cell::new("From"), Cell::new(source)])
                        .add_row(vec![Cell::new("To"), Cell::new(destination)])
                        .add_row(vec![
                            Cell::new("Amount"),
                            Cell::new(format!(
                                "{} lamports ({} SOL)",
                                lamports,
                                crate::misc::helpers::lamports_to_sol(lamports)
                            )),
                        ]);
                }
            }
            "createAccount" => {
                if let (Some(source), Some(new_account), Some(lamports), Some(space), Some(owner)) = (
                    info_map.get("source").and_then(|v| v.as_str()),
                    info_map.get("newAccount").and_then(|v| v.as_str()),
                    info_map.get("lamports").and_then(|v| v.as_u64()),
                    info_map.get("space").and_then(|v| v.as_u64()),
                    info_map.get("owner").and_then(|v| v.as_str()),
                ) {
                    table
                        .add_row(vec![Cell::new("Funder"), Cell::new(source)])
                        .add_row(vec![Cell::new("New Account"), Cell::new(new_account)])
                        .add_row(vec![
                            Cell::new("Lamports"),
                            Cell::new(format!(
                                "{} lamports ({} SOL)",
                                lamports,
                                crate::misc::helpers::lamports_to_sol(lamports)
                            )),
                        ])
                        .add_row(vec![
                            Cell::new("Space"),
                            Cell::new(format!("{} bytes", space)),
                        ])
                        .add_row(vec![Cell::new("Owner"), Cell::new(owner)]);
                }
            }
            "assign" => {
                if let (Some(account), Some(owner)) = (
                    info_map.get("account").and_then(|v| v.as_str()),
                    info_map.get("owner").and_then(|v| v.as_str()),
                ) {
                    table
                        .add_row(vec![Cell::new("Account"), Cell::new(account)])
                        .add_row(vec![Cell::new("Owner"), Cell::new(owner)]);
                }
            }
            "allocate" => {
                if let (Some(account), Some(space)) = (
                    info_map.get("account").and_then(|v| v.as_str()),
                    info_map.get("space").and_then(|v| v.as_u64()),
                ) {
                    table
                        .add_row(vec![Cell::new("Account"), Cell::new(account)])
                        .add_row(vec![
                            Cell::new("Space"),
                            Cell::new(format!("{} bytes", space)),
                        ]);
                }
            }
            _ => {
                // Generic display for other system instructions
                for (key, value) in info_map {
                    let display_value = match value {
                        Value::Object(_) => serde_json::to_string_pretty(value)?,
                        Value::Array(_) => serde_json::to_string(value)?,
                        _ => value.to_string().trim_matches('"').to_string(),
                    };
                    table.add_row(vec![Cell::new(key), Cell::new(display_value)]);
                }
            }
        }
    }

    println!("{}\n", table);
    Ok(())
}

fn display_token_instruction(
    ix_type: &str,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new("Token Program")])
        .add_row(vec![Cell::new("Type"), Cell::new(ix_type)]);

    if let Some(Value::Object(info_map)) = info {
        match ix_type {
            "transfer" | "transferChecked" => {
                if let Some(source) = info_map.get("source").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("From"), Cell::new(source)]);
                }
                if let Some(destination) = info_map.get("destination").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("To"), Cell::new(destination)]);
                }

                // Amount - try tokenAmount object first, then fall back to amount field
                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        table.add_row(vec![
                            Cell::new("Amount"),
                            Cell::new(format!("{} (decimals: {})", ui_amount_str, decimals)),
                        ]);
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    table.add_row(vec![Cell::new("Amount"), Cell::new(amount)]);
                }

                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }

                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map.get("authority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                }
            }
            "mintTo" | "mintToChecked" => {
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
                }

                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        table.add_row(vec![
                            Cell::new("Amount"),
                            Cell::new(format!("{} (decimals: {})", ui_amount_str, decimals)),
                        ]);
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    table.add_row(vec![Cell::new("Amount"), Cell::new(amount)]);
                }

                if let Some(authority) = info_map.get("mintAuthority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map
                    .get("multisigMintAuthority")
                    .and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Mint Authority"), Cell::new(authority)]);
                }
            }
            "burn" | "burnChecked" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }

                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        table.add_row(vec![
                            Cell::new("Amount"),
                            Cell::new(format!("{} (decimals: {})", ui_amount_str, decimals)),
                        ]);
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    table.add_row(vec![Cell::new("Amount"), Cell::new(amount)]);
                }

                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map.get("authority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                }
            }
            "initializeMint" | "initializeMint2" => {
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }
                if let Some(decimals) = info_map.get("decimals").and_then(|v| v.as_u64()) {
                    table.add_row(vec![Cell::new("Decimals"), Cell::new(decimals)]);
                }
                if let Some(mint_authority) = info_map.get("mintAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Mint Authority"), Cell::new(mint_authority)]);
                }
                if let Some(freeze_authority) =
                    info_map.get("freezeAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![
                        Cell::new("Freeze Authority"),
                        Cell::new(freeze_authority),
                    ]);
                }
            }
            "initializeAccount" | "initializeAccount2" | "initializeAccount3" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Owner"), Cell::new(owner)]);
                }
            }
            "closeAccount" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
                }
                if let Some(destination) = info_map.get("destination").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Destination"), Cell::new(destination)]);
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Owner"), Cell::new(owner)]);
                }
            }
            _ => {
                // Generic display for other token instructions
                for (key, value) in info_map {
                    let display_value = match value {
                        Value::Object(_) => serde_json::to_string_pretty(value)?,
                        Value::Array(_) => serde_json::to_string(value)?,
                        _ => value.to_string().trim_matches('"').to_string(),
                    };
                    table.add_row(vec![Cell::new(key), Cell::new(display_value)]);
                }
            }
        }
    }

    println!("{}\n", table);
    Ok(())
}

fn display_ata_instruction(ix_type: &str, info: Option<&serde_json::Value>) -> anyhow::Result<()> {
    use serde_json::Value;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Program"),
            Cell::new("Associated Token Account Program"),
        ])
        .add_row(vec![Cell::new("Type"), Cell::new(ix_type)]);

    if let Some(Value::Object(info_map)) = info {
        match ix_type {
            "create" => {
                if let Some(source) = info_map.get("source").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Funder"), Cell::new(source)]);
                }
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("ATA"), Cell::new(account)]);
                }
                if let Some(wallet) = info_map.get("wallet").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Owner"), Cell::new(wallet)]);
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }
            }
            _ => {
                for (key, value) in info_map {
                    if key == "systemProgram" || key == "tokenProgram" {
                        continue;
                    }

                    let display_value = match value {
                        Value::String(s) => s.clone(),
                        Value::Object(_) => serde_json::to_string_pretty(value)?,
                        Value::Array(_) => serde_json::to_string(value)?,
                        _ => value.to_string(),
                    };
                    table.add_row(vec![Cell::new(key), Cell::new(display_value)]);
                }
            }
        }
    }

    println!("{}\n", table);
    Ok(())
}

fn display_memo_instruction(info: Option<&serde_json::Value>) -> anyhow::Result<()> {
    use serde_json::Value;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new("Memo Program")]);

    if let Some(Value::String(memo)) = info {
        table.add_row(vec![Cell::new("Memo"), Cell::new(memo)]);
    }

    println!("{}\n", table);
    Ok(())
}

fn display_generic_parsed(
    parsed_ix: &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new(&parsed_ix.program)])
        .add_row(vec![
            Cell::new("Program ID"),
            Cell::new(&parsed_ix.program_id),
        ]);

    println!("{}", table);
    println!("{}", style("Parsed Data:").dim());
    println!("{}\n", serde_json::to_string_pretty(&parsed_ix.parsed)?);
    Ok(())
}

fn display_partially_decoded_instruction(
    partial_ix: &solana_transaction_status::UiPartiallyDecodedInstruction,
) -> anyhow::Result<()> {
    if partial_ix.program_id == COMPUTE_BUDGET_PROGRAM_ID {
        return display_compute_budget_instruction(&partial_ix.data);
    }
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Program ID"),
            Cell::new(&partial_ix.program_id),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new("Unknown Program")])
        .add_row(vec![
            Cell::new("Accounts"),
            Cell::new(partial_ix.accounts.len()),
        ])
        .add_row(vec![
            Cell::new("Data (Base58)"),
            Cell::new(&partial_ix.data),
        ]);

    println!("{}\n", table);
    Ok(())
}
fn display_compute_budget_instruction(data_base58: &str) -> anyhow::Result<()> {
    let data = bs58::decode(data_base58).into_vec()?;

    if data.is_empty() {
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Program"),
            Cell::new("Compute Budget Program"),
        ]);

    match data[0] {
        0 => {
            // RequestHeapFrame
            if data.len() >= 5 {
                let bytes = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                table
                    .add_row(vec![Cell::new("Type"), Cell::new("Request Heap Frame")])
                    .add_row(vec![
                        Cell::new("Heap Frame Size"),
                        Cell::new(format!("{} bytes", bytes)),
                    ]);
            }
        }
        1 => {
            // SetComputeUnitLimit
            if data.len() >= 5 {
                let units = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                table
                    .add_row(vec![Cell::new("Type"), Cell::new("Set Compute Unit Limit")])
                    .add_row(vec![
                        Cell::new("Compute Units"),
                        Cell::new(format!("{}", units)),
                    ]);
            }
        }
        2 => {
            // SetComputeUnitPrice
            // Handle both u64 (9 bytes) and u32 (5 bytes) formats
            let micro_lamports = if data.len() >= 9 {
                u64::from_le_bytes([
                    data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                ])
            } else if data.len() >= 5 {
                // Pad u32 to u64
                u64::from_le_bytes([
                    data[1], data[2], data[3], data[4], 0, 0, 0, 0, // Zero-pad the upper bytes
                ])
            } else {
                0 // Shouldn't happen, but safe fallback
            };

            table
                .add_row(vec![Cell::new("Type"), Cell::new("Set Compute Unit Price")])
                .add_row(vec![
                    Cell::new("Priority Fee"),
                    Cell::new(format!("{} micro-lamports/CU", micro_lamports)),
                ]);
        }
        3 => {
            // SetLoadedAccountsDataSizeLimit
            if data.len() >= 5 {
                let bytes = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                table
                    .add_row(vec![
                        Cell::new("Type"),
                        Cell::new("Set Loaded Accounts Data Size Limit"),
                    ])
                    .add_row(vec![
                        Cell::new("Size Limit"),
                        Cell::new(format!("{} bytes", bytes)),
                    ]);
            }
        }
        _ => {
            // Unknown or future instruction types
            table
                .add_row(vec![
                    Cell::new("Type"),
                    Cell::new("Unknown Compute Budget Instruction"),
                ])
                .add_row(vec![Cell::new("Discriminator"), Cell::new(data[0])])
                .add_row(vec![Cell::new("Data (Base58)"), Cell::new(data_base58)]);
        }
    }

    println!("{}\n", table);
    Ok(())
}

fn display_compiled_instruction(
    compiled_ix: &solana_transaction_status::UiCompiledInstruction,
) -> anyhow::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Program Index"),
            Cell::new(compiled_ix.program_id_index),
        ])
        .add_row(vec![
            Cell::new("Accounts"),
            Cell::new(compiled_ix.accounts.len()),
        ])
        .add_row(vec![
            Cell::new("Data (Base58)"),
            Cell::new(&compiled_ix.data),
        ]);

    println!("{}\n", table);
    Ok(())
}

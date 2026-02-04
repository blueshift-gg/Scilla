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
    inquire::Confirm,
    ptree::{TreeBuilder, print_tree},
    solana_account_decoder::UiAccount,
    solana_rpc_client_api::config::RpcTransactionConfig,
    solana_signature::Signature,
    solana_transaction_status::{
        EncodedTransaction, UiInnerInstructions, UiInstruction, UiMessage, UiParsedInstruction,
        UiTransactionEncoding,
    },
    std::fmt,
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
                let parse_instructions =
                    Confirm::new("Do you want to parse instructions for this transaction?")
                        .with_default(true)
                        .prompt()
                        .unwrap_or(true);
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_transaction(ctx, &signature, parse_instructions),
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
    parse_instructions: bool,
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

            if parse_instructions {
                process_parse_instructions(parsed_msg)?;
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

fn process_parse_instructions(
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
    let mut tree = TreeBuilder::new(format!("Instruction {}", idx));

    match ui_instruction {
        UiInstruction::Parsed(ui_parsed_ix) => match ui_parsed_ix {
            UiParsedInstruction::Parsed(parsed_ix) => {
                display_parsed_instruction(&mut tree, parsed_ix)?;
            }
            UiParsedInstruction::PartiallyDecoded(partial_ix) => {
                display_partially_decoded_instruction(&mut tree, partial_ix)?;
            }
        },
        UiInstruction::Compiled(compiled_ix) => {
            display_compiled_instruction(&mut tree, compiled_ix)?;
        }
    }

    print_tree(&tree.build())?;
    println!(); // Add spacing between instructions
    Ok(())
}

fn display_parsed_instruction(
    tree: &mut TreeBuilder,
    parsed_ix: &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    use serde_json::Value;

    tree.add_empty_child(format!("Program: {}", style(&parsed_ix.program).yellow()));

    if parsed_ix.program == "spl-memo"
        && let Value::String(memo) = &parsed_ix.parsed
    {
        return display_memo_instruction(tree, Some(&Value::String(memo.clone())));
    }

    let Value::Object(parsed_map) = &parsed_ix.parsed else {
        return display_generic_parsed(tree, parsed_ix);
    };

    let ix_type = parsed_map
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tree.add_empty_child(format!("Type: {}", style(ix_type).green()));

    match parsed_ix.program.as_str() {
        "system" => display_system_instruction(tree, ix_type, parsed_map.get("info"))?,
        "spl-token" | "spl-token-2022" => {
            display_token_instruction(tree, ix_type, parsed_map.get("info"))?
        }
        "spl-associated-token-account" => display_ata_instruction(tree, parsed_map.get("info"))?,
        _ => display_generic_parsed(tree, parsed_ix)?,
    }

    Ok(())
}

fn display_system_instruction(
    tree: &mut TreeBuilder,
    ix_type: &str,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    if let Some(Value::Object(info_map)) = info {
        match ix_type {
            "transfer" => {
                if let Some(source) = info_map.get("source").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "From", style(source).cyan()));
                }
                if let Some(destination) = info_map.get("destination").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "To", style(destination).cyan()));
                }
                if let Some(lamports) = info_map.get("lamports").and_then(|v| v.as_u64()) {
                    tree.add_empty_child(format!(
                        "Amount: {} lamports ({} SOL)",
                        lamports,
                        crate::misc::helpers::lamports_to_sol(lamports)
                    ));
                }
            }
            "createAccount" => {
                if let Some(source) = info_map.get("source").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "Source", style(source).cyan()));
                }
                if let Some(new_account) = info_map.get("newAccount").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!(
                        "{}: {}",
                        "New Account",
                        style(new_account).cyan()
                    ));
                }
                if let Some(lamports) = info_map.get("lamports").and_then(|v| v.as_u64()) {
                    tree.add_empty_child(format!("Lamports: {}", lamports));
                }
                if let Some(space) = info_map.get("space").and_then(|v| v.as_u64()) {
                    tree.add_empty_child(format!("Space: {} bytes", space));
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Owner: {}", owner));
                }
            }
            "assign" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Account: {}", account));
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Owner: {}", owner));
                }
            }
            "allocate" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Account: {}", account));
                }
                if let Some(space) = info_map.get("space").and_then(|v| v.as_u64()) {
                    tree.add_empty_child(format!("Space: {} bytes", space));
                }
            }
            _ => {
                // Generic display
                for (key, value) in info_map {
                    if let Some(s) = value.as_str() {
                        tree.add_empty_child(format!("{}: {}", key, s));
                    }
                }
            }
        }
    }

    Ok(())
}

fn display_token_instruction(
    tree: &mut TreeBuilder,
    ix_type: &str,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    if let Some(Value::Object(info_map)) = info {
        match ix_type {
            "transfer" | "transferChecked" => {
                if let Some(source) = info_map.get("source").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", style("From"), style(source).cyan()));
                }
                if let Some(destination) = info_map.get("destination").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", style("To"), style(destination).cyan()));
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "Mint", style(mint).cyan()));
                }

                // Amount
                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tree.add_empty_child(format!(
                            "Amount: {} (decimals: {})",
                            ui_amount_str, decimals
                        ));
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    tree.add_empty_child(format!("Amount: {}", amount));
                }
            }
            "mintTo" | "mintToChecked" => {
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "Mint", style(mint).cyan()));
                }
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "To", style(account).cyan()));
                }

                // Amount
                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tree.add_empty_child(format!(
                            "Amount: {} (decimals: {})",
                            ui_amount_str, decimals
                        ));
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    tree.add_empty_child(format!("Amount: {}", amount));
                }
            }
            "burn" | "burnChecked" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "Burn From", style(account).cyan()));
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("{}: {}", "Mint", style(mint).cyan()));
                }

                // Amount
                if let Some(token_amount) = info_map.get("tokenAmount") {
                    if let Some(ui_amount_str) =
                        token_amount.get("uiAmountString").and_then(|v| v.as_str())
                    {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tree.add_empty_child(format!(
                            "Amount: {} (decimals: {})",
                            ui_amount_str, decimals
                        ));
                    }
                } else if let Some(amount_val) = info_map.get("amount") {
                    let amount = if let Some(s) = amount_val.as_str() {
                        s.to_string()
                    } else if let Some(n) = amount_val.as_u64() {
                        n.to_string()
                    } else {
                        amount_val.to_string()
                    };
                    tree.add_empty_child(format!("Amount: {}", amount));
                }
            }
            "initializeMint" | "initializeMint2" => {
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Mint: {}", style(mint).cyan()));
                }
                if let Some(decimals) = info_map.get("decimals").and_then(|v| v.as_u64()) {
                    tree.add_empty_child(format!("Decimals: {}", decimals));
                }
                if let Some(mint_authority) = info_map.get("mintAuthority").and_then(|v| v.as_str())
                {
                    tree.add_empty_child(format!("Mint Authority: {}", mint_authority));
                }
                if let Some(freeze_authority) =
                    info_map.get("freezeAuthority").and_then(|v| v.as_str())
                {
                    tree.add_empty_child(format!("Freeze Authority: {}", freeze_authority));
                }
            }
            "initializeAccount" | "initializeAccount2" | "initializeAccount3" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Account: {}", account));
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Mint: {}", mint));
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Owner: {}", owner));
                }
            }
            "closeAccount" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Account: {}", account));
                }
                if let Some(destination) = info_map.get("destination").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Destination: {}", destination));
                }
                if let Some(owner) = info_map.get("owner").and_then(|v| v.as_str()) {
                    tree.add_empty_child(format!("Owner: {}", owner));
                }
            }
            _ => {
                // Generic display
                for (key, value) in info_map {
                    let display_value = match value {
                        Value::String(s) => s.clone(),
                        _ => value.to_string().trim_matches('"').to_string(),
                    };
                    tree.add_empty_child(format!("{}: {}", key, display_value));
                }
            }
        }
    }

    Ok(())
}

fn display_ata_instruction(
    tree: &mut TreeBuilder,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    if let Some(Value::Object(info_map)) = info {
        for (key, value) in info_map {
            if key == "systemProgram" || key == "tokenProgram" {
                continue;
            }

            let display_value = match value {
                Value::String(s) => s.clone(),
                _ => value.to_string().trim_matches('"').to_string(),
            };

            let label = match key.as_str() {
                "source" => "Source",
                "account" => "ATA",
                "wallet" => "Owner",
                "mint" => "Mint",
                _ => key,
            };

            tree.add_empty_child(format!("{}: {}", label, style(&display_value).cyan()));
        }
    }

    Ok(())
}

fn display_memo_instruction(
    tree: &mut TreeBuilder,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    use serde_json::Value;

    if let Some(Value::String(memo)) = info {
        tree.add_empty_child(format!("Memo: {}", memo));
    }

    Ok(())
}

fn display_generic_parsed(
    tree: &mut TreeBuilder,
    parsed_ix: &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    tree.add_empty_child(format!("Program ID: {}", &parsed_ix.program_id));
    tree.add_empty_child(format!(
        "Parsed Data: {}",
        serde_json::to_string_pretty(&parsed_ix.parsed)?
    ));

    Ok(())
}

fn display_partially_decoded_instruction(
    tree: &mut TreeBuilder,
    partial_ix: &solana_transaction_status::UiPartiallyDecodedInstruction,
) -> anyhow::Result<()> {
    // Check if this is a Compute Budget instruction
    if partial_ix.program_id == COMPUTE_BUDGET_PROGRAM_ID {
        tree.add_empty_child(format!("Program: {}", style("Compute Budget").yellow()));
        display_compute_budget_instruction(tree, &partial_ix.data)?;
        return Ok(());
    }

    // Unknown program
    tree.add_empty_child(format!(
        "Program: {} {}",
        style("Unknown").red(),
        &partial_ix.program_id
    ));
    tree.add_empty_child(format!("Accounts: {}", partial_ix.accounts.len()));
    tree.add_empty_child(format!("Data: {}", partial_ix.data));

    Ok(())
}

fn display_compute_budget_instruction(
    tree: &mut TreeBuilder,
    data_base58: &str,
) -> anyhow::Result<()> {
    let data = bs58::decode(data_base58).into_vec()?;

    if data.is_empty() {
        return Ok(());
    }

    match data[0] {
        0 => {
            if data.len() >= 5 {
                let bytes = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                tree.add_empty_child(format!("Type: {}", style("Request Heap Frame").green()));
                tree.add_empty_child(format!("Heap Frame Size: {} bytes", bytes));
            }
        }
        1 => {
            if data.len() >= 5 {
                let units = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                tree.add_empty_child(format!("Type: {}", style("Set Compute Unit Limit").green()));
                tree.add_empty_child(format!("Compute Units: {}", units));
            }
        }
        2 => {
            let micro_lamports = if data.len() >= 9 {
                u64::from_le_bytes([
                    data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                ])
            } else if data.len() >= 5 {
                u64::from_le_bytes([data[1], data[2], data[3], data[4], 0, 0, 0, 0])
            } else {
                0
            };
            tree.add_empty_child(format!("Type: {}", style("Set Compute Unit Price").green()));
            tree.add_empty_child(format!(
                "Priority Fee: {} micro-lamports/CU",
                micro_lamports
            ));
        }
        3 => {
            if data.len() >= 5 {
                let bytes = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                tree.add_empty_child(format!(
                    "Type: {}",
                    style("Set Loaded Accounts Data Size Limit").green()
                ));
                tree.add_empty_child(format!("Size Limit: {} bytes", bytes));
            }
        }
        _ => {
            tree.add_empty_child(format!("Unknown instruction type: {}", data[0]));
        }
    }

    Ok(())
}

fn display_compiled_instruction(
    tree: &mut TreeBuilder,
    compiled_ix: &solana_transaction_status::UiCompiledInstruction,
) -> anyhow::Result<()> {
    tree.add_empty_child(format!("Type: {}", "Compiled"));
    tree.add_empty_child(format!("Program Index: {}", compiled_ix.program_id_index));
    tree.add_empty_child(format!("Accounts: {}", compiled_ix.accounts.len()));
    tree.add_empty_child(format!("Data: {}", compiled_ix.data));

    Ok(())
}

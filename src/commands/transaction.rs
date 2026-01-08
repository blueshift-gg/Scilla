use {
    crate::{
        commands::CommandFlow,
        context::ScillaContext,
        misc::helpers::{bincode_deserialize, decode_base58, decode_base64},
        prompt::{prompt_input_data, prompt_select_data},
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::Confirm,
    serde_json::Value,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::config::RpcTransactionConfig,
    solana_signature::Signature,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_status::{
        EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,
    },
    std::{collections::HashMap, fmt, path::PathBuf, str::FromStr},
};

#[derive(Debug, Clone)]
pub enum TransactionCommand {
    CheckConfirmation,
    FetchStatus,
    FetchTransaction,
    SendTransaction,
    ParseInstruction,
    GoBack,
}

impl TransactionCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            Self::CheckConfirmation => "Checking transaction confirmation…",
            Self::FetchStatus => "Fetching transaction status…",
            Self::FetchTransaction => "Fetching full transaction data…",
            Self::SendTransaction => "Sending transaction…",
            Self::ParseInstruction => "Parsing instruction data…",
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
            Self::ParseInstruction => "Parse Instruction",
            Self::GoBack => "Go back",
        })
    }
}

impl TransactionCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> CommandFlow<()> {
        match self {
            TransactionCommand::CheckConfirmation => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                show_spinner(
                    self.spinner_msg(),
                    process_check_confirmation(ctx, &signature),
                )
                .await;
            }
            TransactionCommand::FetchStatus => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_transaction_status(ctx, &signature),
                )
                .await;
            }
            TransactionCommand::FetchTransaction => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_transaction(ctx, &signature),
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

                let encoding = prompt_select_data(
                    "Select encoding format:",
                    vec![UiTransactionEncoding::Base64, UiTransactionEncoding::Base58],
                );

                let encoded_tx: String = prompt_input_data("Enter encoded transaction:");

                show_spinner(
                    self.spinner_msg(),
                    process_send_transaction(ctx, encoding, &encoded_tx),
                )
                .await;
            }

            TransactionCommand::ParseInstruction => {
                let signature: Signature = prompt_input_data("Enter transaction signature:");

                let use_custom_idl = Confirm::new("Use custom IDL path for this parse?")
                    .with_default(false)
                    .prompt()
                    .unwrap_or(false);

                let custom_idl_path = if use_custom_idl {
                    Some(prompt_input_data::<String>("Enter IDL file path:"))
                } else {
                    None
                };

                show_spinner(
                    self.spinner_msg(),
                    process_parse_instruction(ctx, &signature, custom_idl_path),
                )
                .await;
            }
            TransactionCommand::GoBack => return CommandFlow::GoBack,
        }

        CommandFlow::Process(())
    }
}

async fn process_check_confirmation(
    ctx: &ScillaContext,
    signature: &Signature,
) -> anyhow::Result<()> {
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
    println!("{}", table);

    Ok(())
}

async fn process_fetch_transaction_status(
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
    println!("{}", table);

    Ok(())
}

async fn process_fetch_transaction(
    ctx: &ScillaContext,
    signature: &Signature,
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
    println!("{}", table);

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

            println!("{}", msg_table);

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
                println!("{}", accounts_table);
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

            println!("{}", msg_table);

            if !raw_msg.account_keys.is_empty() {
                println!("\n{}", style("ACCOUNT KEYS").cyan().bold());
                for (idx, key) in raw_msg.account_keys.iter().enumerate() {
                    println!("  {}. {}", idx, key);
                }
            }
        }
    }

    Ok(())
}

async fn process_send_transaction(
    ctx: &ScillaContext,
    encoding: UiTransactionEncoding,
    encoded_tx: &str,
) -> anyhow::Result<()> {
    let tx_bytes = match encoding {
        UiTransactionEncoding::Base64 => decode_base64(encoded_tx)?,
        UiTransactionEncoding::Base58 => decode_base58(encoded_tx)?,
        _ => unreachable!("The available encoding options are Base64 and Base58"),
    };

    let tx: VersionedTransaction =
        bincode_deserialize(&tx_bytes, "encoded transaction to VersionedTransaction")?;

    let signature = ctx.rpc().send_transaction(&tx).await?;

    println!(
        "{} {}",
        style("Transaction sent successfully!").green().bold(),
        style(signature).cyan()
    );

    Ok(())
}

async fn process_parse_instruction(
    ctx: &ScillaContext,
    signature: &Signature,
    custom_idl_path: Option<String>,
) -> anyhow::Result<()> {
    // Fetch the transaction with JsonParsed encoding
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

    let EncodedTransaction::Json(ui_tx) = &tx.transaction.transaction else {
        anyhow::bail!("Transaction encoding is not JSON");
    };

    let UiMessage::Parsed(parsed_msg) = &ui_tx.message else {
        anyhow::bail!("Transaction message is not parsed");
    };

    if parsed_msg.instructions.is_empty() {
        println!("{}", style("No instructions found in transaction").yellow());
        return Ok(());
    }

    println!(
        "\n{} {}",
        style("Found").green().bold(),
        style(format!("{} instruction(s)", parsed_msg.instructions.len())).cyan()
    );

    let config = crate::config::ScillaConfig::load()?;

    let idl_parser = if let Some(custom_path) = custom_idl_path {
        IdlParser::new_with_custom_path(&custom_path)
    } else {
        IdlParser::new(&config.idl)
    };

    // Display each instruction
    for (idx, ui_instruction) in parsed_msg.instructions.iter().enumerate() {
        display_ui_instruction_with_idl(idx, ui_instruction, &idl_parser).await?;
    }

    Ok(())
}
async fn display_ui_instruction_with_idl(
    idx: usize,
    ui_instruction: &UiInstruction,
    idl_parser: &IdlParser,
) -> anyhow::Result<()> {
    println!("\n{}", style(format!("INSTRUCTION #{}", idx)).cyan().bold());

    match ui_instruction {
        UiInstruction::Parsed(ui_parsed_ix) => match ui_parsed_ix {
            UiParsedInstruction::Parsed(parsed_ix) => display_parsed_instruction(parsed_ix),
            UiParsedInstruction::PartiallyDecoded(partial_ix) => {
                let program_id = Pubkey::from_str(&partial_ix.program_id)?;
                let instruction_data = bs58::decode(&partial_ix.data).into_vec()?;

                if let Some(custom_data) = idl_parser
                    .try_parse_custom(&program_id, &instruction_data)
                    .await
                {
                    display_custom_instruction(&program_id, &custom_data)
                } else {
                    display_partially_decoded_instruction(partial_ix)
                }
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
        "spl-memo" => display_memo_instruction(info),
        _ => display_generic_parsed(parsed_ix),
    }
}

fn display_system_instruction(
    ix_type: &str,
    info: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    if let Some(info_val) = info {
        println!("\n{}", style("DEBUG - Full Info:").yellow());
        println!("{}", serde_json::to_string_pretty(info_val)?);
    }

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
            _ => {
                // Generic display for other system instructions
                for (key, value) in info_map {
                    table.add_row(vec![Cell::new(key), Cell::new(value.to_string())]);
                }
            }
        }
    }

    println!("{}", table);
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
                // Source and Destination
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
                        // Fallback to JSON string representation
                        amount_val.to_string()
                    };
                    table.add_row(vec![Cell::new("Amount"), Cell::new(amount)]);
                }

                // Mint (for transferChecked)
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }

                // Authority
                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map.get("authority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                }

                // Signers (for multisig)
                if let Some(Value::Array(signers)) = info_map.get("signers") {
                    let signer_list: Vec<String> = signers
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    if !signer_list.is_empty() {
                        table.add_row(vec![
                            Cell::new("Signers"),
                            Cell::new(signer_list.join(", ")),
                        ]);
                    }
                }
            }
            "mintTo" | "mintToChecked" => {
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
                }
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
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
                        // Fallback to JSON string representation
                        amount_val.to_string()
                    };
                    table.add_row(vec![Cell::new("Amount"), Cell::new(amount)]);
                }

                // Authority
                if let Some(authority) = info_map.get("mintAuthority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map
                    .get("multisigMintAuthority")
                    .and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Mint Authority"), Cell::new(authority)]);
                }
            }
            "burn" => {
                if let Some(account) = info_map.get("account").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Account"), Cell::new(account)]);
                }
                if let Some(mint) = info_map.get("mint").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Mint"), Cell::new(mint)]);
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

                // Authority
                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str())
                {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                } else if let Some(authority) = info_map.get("authority").and_then(|v| v.as_str()) {
                    table.add_row(vec![Cell::new("Authority"), Cell::new(authority)]);
                }
            }
            _ => {
                // Generic display for other token instructions
                for (key, value) in info_map {
                    let display_value = match value {
                        Value::Object(_) => serde_json::to_string_pretty(value)?,
                        Value::Array(_) => serde_json::to_string(value)?,
                        _ => value.to_string(),
                    };
                    table.add_row(vec![Cell::new(key), Cell::new(display_value)]);
                }
            }
        }
    }

    println!("{}", table);
    Ok(())
}

fn display_memo_instruction(info: Option<&serde_json::Value>) -> anyhow::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new("Memo Program")]);

    if let Some(Value::Object(info_map)) = info
        && let Some(memo) = info_map.get("memo").and_then(|v| v.as_str())
    {
        table.add_row(vec![Cell::new("Memo"), Cell::new(memo)]);
    }

    println!("{}", table);
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
    println!("\n{}", style("Parsed Data:").dim());
    println!("{}", serde_json::to_string_pretty(&parsed_ix.parsed)?);
    println!();
    Ok(())
}

fn display_partially_decoded_instruction(
    partial_ix: &solana_transaction_status::UiPartiallyDecodedInstruction,
) -> anyhow::Result<()> {
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
        .add_row(vec![
            Cell::new("Accounts"),
            Cell::new(partial_ix.accounts.len()),
        ])
        .add_row(vec![
            Cell::new("Data (Base58)"),
            Cell::new(&partial_ix.data),
        ]);

    println!("{}", table);

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

    println!("{}", table);
    Ok(())
}

fn display_custom_instruction(
    program_id: &Pubkey,
    custom_data: &CustomInstructionData,
) -> anyhow::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Program"),
            Cell::new(&custom_data.program_name),
        ])
        .add_row(vec![
            Cell::new("Program ID"),
            Cell::new(program_id.to_string()),
        ])
        .add_row(vec![
            Cell::new("Instruction"),
            Cell::new(&custom_data.instruction_name),
        ]);

    for (key, value) in &custom_data.args {
        let display_value = match value {
            Value::Object(_) | Value::Array(_) => serde_json::to_string_pretty(value)?,
            _ => value.to_string().trim_matches('"').to_string(),
        };
        table.add_row(vec![Cell::new(key), Cell::new(display_value)]);
    }

    println!("{}", table);
    Ok(())
}

struct IdlParser {
    idl_path: PathBuf,
    is_file: bool,
}

impl IdlParser {
    fn new(config: &crate::config::IdlConfig) -> Self {
        Self {
            idl_path: config.custom_idl_path.clone(),
            is_file: false,
        }
    }

    fn new_with_custom_path(path: &str) -> Self {
        let expanded = crate::config::expand_tilde(path);
        let is_file = expanded.is_file();

        Self {
            idl_path: expanded,
            is_file,
        }
    }

    async fn try_parse_custom(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
    ) -> Option<CustomInstructionData> {
        let idl_file = if self.is_file {
            self.idl_path.clone()
        } else {
            self.idl_path.join(format!("{}.json", program_id))
        };

        if !idl_file.exists() {
            return None;
        }

        let idl_json = tokio::fs::read_to_string(&idl_file).await.ok()?;
        let idl: serde_json::Value = serde_json::from_str(&idl_json).ok()?;

        let program_name = idl.get("name")?.as_str()?.to_string();

        if instruction_data.len() < 8 {
            return None;
        }

        let discriminator = &instruction_data[0..8];

        let instructions = idl.get("instructions")?.as_array()?;

        // Try to match discriminator with multiple schemes
        for ix in instructions {
            let ix_name = ix.get("name")?.as_str()?;

            // Generate all possible discriminators for this instruction
            let possible_discriminators = generate_all_discriminators(ix_name);

            // Check if any match
            if possible_discriminators.iter().any(|d| d == discriminator) {
                let args_data = &instruction_data[8..];
                let mut args = HashMap::new();
                args.insert(
                    "data_hex".to_string(),
                    Value::String(hex::encode(args_data)),
                );

                if let Some(args_array) = ix.get("args").and_then(|a| a.as_array()) {
                    let arg_names: Vec<String> = args_array
                        .iter()
                        .filter_map(|arg| arg.get("name")?.as_str().map(String::from))
                        .collect();

                    if !arg_names.is_empty() {
                        args.insert(
                            "expected_args".to_string(),
                            Value::Array(arg_names.into_iter().map(Value::String).collect()),
                        );
                    }
                }

                return Some(CustomInstructionData {
                    program_name,
                    instruction_name: ix_name.to_string(),
                    args,
                });
            }
        }

        None
    }
}

struct CustomInstructionData {
    program_name: String,
    instruction_name: String,
    args: HashMap<String, Value>,
}

/// Generate all possible discriminator schemes for an instruction name
fn generate_all_discriminators(ix_name: &str) -> Vec<[u8; 8]> {
    let mut discriminators = Vec::new();

    // Scheme 1: Anchor style - global:camelCase
    discriminators.push(compute_discriminator(&format!("global:{}", ix_name)));

    // Scheme 2: Anchor style - global:PascalCase
    let pascal_case = to_pascal_case(ix_name);
    discriminators.push(compute_discriminator(&format!("global:{}", pascal_case)));

    // Scheme 3: Shank style - global:snake_case
    let snake_case = to_snake_case(ix_name);
    discriminators.push(compute_discriminator(&format!("global:{}", snake_case)));

    // Scheme 4: Just the name without prefix
    discriminators.push(compute_discriminator(ix_name));

    // Scheme 5: Just PascalCase without prefix
    discriminators.push(compute_discriminator(&pascal_case));

    // Scheme 6: Just snake_case without prefix
    discriminators.push(compute_discriminator(&snake_case));

    discriminators
}

fn compute_discriminator(preimage: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(preimage.as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

fn to_pascal_case(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap().to_uppercase().to_string();
    let rest: String = chars.collect();

    format!("{}{}", first, rest)
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

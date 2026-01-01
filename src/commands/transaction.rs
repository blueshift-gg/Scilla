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
    solana_rpc_client_api::config::RpcTransactionConfig,
    solana_signature::Signature,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_status::{
        EncodedTransaction, 
        UiMessage, 
        UiTransactionEncoding,
        UiInstruction,
        UiParsedInstruction,
    },
    std::fmt,
    serde_json::Value,
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

                show_spinner(
                    self.spinner_msg(),
                    process_parse_instruction(ctx, &signature),
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
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
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
    let status = ctx.rpc().get_signature_statuses(&[*signature]).await?;

    let Some(Some(tx_status)) = status.value.first() else {
        anyhow::bail!("Transaction not found");
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Signature"), Cell::new(signature)])
        .add_row(vec![Cell::new("Slot"), Cell::new(tx_status.slot)])
        .add_row(vec![
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
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
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
                    Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
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
                    Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
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

    // Display each instruction
    for (idx, ui_instruction) in parsed_msg.instructions.iter().enumerate() {
        display_ui_instruction(idx, ui_instruction)?;
    }

    Ok(())
}

fn display_ui_instruction(idx: usize, ui_instruction: &UiInstruction) -> anyhow::Result<()> {
    println!("\n{}", style(format!("INSTRUCTION #{}", idx)).cyan().bold());

    match ui_instruction {
        UiInstruction::Parsed(ui_parsed_ix) => {
      
            match ui_parsed_ix {
                UiParsedInstruction::Parsed(parsed_ix) => {
                   
                    display_parsed_instruction(parsed_ix)
                }
                UiParsedInstruction::PartiallyDecoded(partial_ix) => {
                    display_partially_decoded_instruction(partial_ix)
                }
            }
        }
        UiInstruction::Compiled(compiled_ix) => {
            display_compiled_instruction(compiled_ix)
        }
    }
}

fn display_parsed_instruction(
    parsed_ix:  &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    use serde_json::Value;

    // Try to parse known native programs
    let Value::Object(parsed_map) = &parsed_ix.parsed else {
        // If not an object, just show generic display
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
                        .add_row(vec![Cell::new("Space"), Cell::new(format!("{} bytes", space))])
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
                    if let Some(ui_amount_str) = token_amount.get("uiAmountString").and_then(|v| v.as_str()) {
                        let decimals = token_amount.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0);
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
                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str()) {
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
                    if let Some(ui_amount_str) = token_amount.get("uiAmountString").and_then(|v| v.as_str()) {
                        let decimals = token_amount.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0);
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
                } else if let Some(authority) = info_map.get("multisigMintAuthority").and_then(|v| v.as_str()) {
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
                    if let Some(ui_amount_str) = token_amount.get("uiAmountString").and_then(|v| v.as_str()) {
                        let decimals = token_amount.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0);
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
                if let Some(authority) = info_map.get("multisigAuthority").and_then(|v| v.as_str()) {
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

    if let Some(Value::Object(info_map)) = info {
        if let Some(memo) = info_map.get("memo").and_then(|v| v.as_str()) {
            table.add_row(vec![Cell::new("Memo"), Cell::new(memo)]);
        }
    }

    println!("{}", table);
    Ok(())
}

fn display_generic_parsed(
    parsed_ix:  &solana_transaction_status::parse_instruction::ParsedInstruction,
) -> anyhow::Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![Cell::new("Program"), Cell::new(&parsed_ix.program)])
        .add_row(vec![Cell::new("Program ID"), Cell::new(&parsed_ix.program_id)]);

    println!("{}", table);
    println!("\n{}", style("Parsed Data:").dim());
    println!("{}", serde_json::to_string_pretty(&parsed_ix.parsed)?);
    print!("\n");
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
        .add_row(vec![Cell::new("Program ID"), Cell::new(&partial_ix.program_id)])
        .add_row(vec![
            Cell::new("Accounts"),
            Cell::new(partial_ix.accounts.len()),
        ])
        .add_row(vec![Cell::new("Data (Base58)"), Cell::new(&partial_ix.data)]);

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
        .add_row(vec![Cell::new("Data (Base58)"), Cell::new(&compiled_ix.data)]);

    println!("{}", table);
    Ok(())
}
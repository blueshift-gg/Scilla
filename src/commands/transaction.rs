use {
    crate::{
        commands::CommandExec,
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::{decode_base58, decode_base64},
        prompt::prompt_data,
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    inquire::Select,
    solana_rpc_client_api::config::RpcTransactionConfig,
    solana_signature::Signature,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_status::{EncodedTransaction, UiMessage, UiTransactionEncoding},
    std::fmt,
};

#[derive(Debug, Clone)]
pub enum TransactionCommand {
    CheckConfirmation,
    FetchStatus,
    FetchTransaction,
    SendTransaction,
    GoBack,
}

#[derive(Debug, Clone)]
enum TransactionEncoding {
    Base64,
    Base58,
}

impl TransactionCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            Self::CheckConfirmation => "Checking transaction confirmation…",
            Self::FetchStatus => "Fetching transaction status…",
            Self::FetchTransaction => "Fetching full transaction data…",
            Self::SendTransaction => "Sending transaction…",
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
            Self::GoBack => "Go Back",
        })
    }
}

impl fmt::Display for TransactionEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Base64 => "Base64",
            Self::Base58 => "Base58",
        })
    }
}

impl TransactionCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            TransactionCommand::CheckConfirmation => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(
                    self.spinner_msg(),
                    process_check_confirmation(ctx, &signature),
                )
                .await?;
            }
            TransactionCommand::FetchStatus => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(self.spinner_msg(), process_fetch_status(ctx, &signature)).await?;
            }
            TransactionCommand::FetchTransaction => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_transaction(ctx, &signature),
                )
                .await?;
            }
            TransactionCommand::SendTransaction => {
                let encoding = Select::new(
                    "Select encoding format:",
                    vec![TransactionEncoding::Base64, TransactionEncoding::Base58],
                )
                .prompt()?;

                let encoded_tx: String = prompt_data("Enter encoded transaction:")?;
                show_spinner(
                    self.spinner_msg(),
                    process_send_transaction(ctx, encoding, encoded_tx),
                )
                .await?;
            }
            TransactionCommand::GoBack => return Ok(CommandExec::GoBack),
        }

        Ok(CommandExec::Process(()))
    }
}

async fn process_check_confirmation(
    ctx: &ScillaContext,
    signature: &Signature,
) -> anyhow::Result<()> {
    let confirmed = ctx.rpc().confirm_transaction(signature).await?;

    let status = if confirmed {
        "Confirmed"
    } else {
        "Not Confirmed"
    };
    let status_color = if confirmed {
        style(status).green()
    } else {
        style(status).yellow()
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Signature"),
            Cell::new(signature.to_string()),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new(status_color.to_string()),
        ]);

    println!("\n{}", style("TRANSACTION CONFIRMATION").green().bold());
    println!("{}", table);

    Ok(())
}

async fn process_fetch_status(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()> {
    let status = ctx.rpc().get_signature_statuses(&[*signature]).await?;

    match status.value.first() {
        Some(Some(tx_status)) => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_header(vec![
                    Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
                ])
                .add_row(vec![
                    Cell::new("Signature"),
                    Cell::new(signature.to_string()),
                ])
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
        }
        Some(None) | None => {
            println!("{}", style("Transaction not found").yellow());
        }
    }

    Ok(())
}

async fn process_fetch_transaction(
    ctx: &ScillaContext,
    signature: &Signature,
) -> anyhow::Result<()> {
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(ctx.rpc().commitment()),
        max_supported_transaction_version: Some(0),
    };

    let tx = ctx
        .rpc()
        .get_transaction_with_config(signature, config)
        .await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Signature"),
            Cell::new(signature.to_string()),
        ])
        .add_row(vec![Cell::new("Slot"), Cell::new(format!("{}", tx.slot))]);

    if let Some(block_time) = tx.block_time {
        table.add_row(vec![
            Cell::new("Block Time"),
            Cell::new(format!("{}", block_time)),
        ]);
    }

    if let Some(meta) = &tx.transaction.meta {
        table.add_row(vec![
            Cell::new("Fee (lamports)"),
            Cell::new(format!("{}", meta.fee)),
        ]);
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

    if let EncodedTransaction::Json(ui_tx) = &tx.transaction.transaction {
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
                        Cell::new(format!("{}", parsed_msg.account_keys.len())),
                    ])
                    .add_row(vec![
                        Cell::new("Recent Blockhash"),
                        Cell::new(parsed_msg.recent_blockhash.clone()),
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
                            Cell::new(format!("{}", idx)),
                            Cell::new(account.pubkey.clone()),
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
                        Cell::new(format!("{}", raw_msg.account_keys.len())),
                    ])
                    .add_row(vec![
                        Cell::new("Recent Blockhash"),
                        Cell::new(raw_msg.recent_blockhash.clone()),
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
    }

    Ok(())
}

async fn process_send_transaction(
    ctx: &ScillaContext,
    encoding: TransactionEncoding,
    encoded_tx: String,
) -> anyhow::Result<()> {
    let tx_bytes = match encoding {
        TransactionEncoding::Base64 => decode_base64(&encoded_tx)?,
        TransactionEncoding::Base58 => decode_base58(&encoded_tx)?,
    };

    let tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

    let signature = ctx.rpc().send_transaction(&tx).await?;

    println!(
        "\n{} {}",
        style("Transaction sent successfully!").green().bold(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

use {
    crate::{
        commands::{CommandFlow, NavigationTarget},
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
    solana_transaction_status::{EncodedTransaction, UiMessage, UiTransactionEncoding},
    std::fmt,
};

#[derive(Debug, Clone, Copy)]
pub enum TransactionCommand {
    CheckConfirmation,
    FetchStatus,
    FetchTransaction,
    SendTransaction,
    GoBack,
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
            Self::GoBack => "Go back",
        })
    }
}

impl TransactionCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> CommandFlow {
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
            TransactionCommand::GoBack => return CommandFlow::NavigateTo(NavigationTarget::MainMenu),
        }

        CommandFlow::Processed
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
    println!("{table}");

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
    println!("{table}");

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

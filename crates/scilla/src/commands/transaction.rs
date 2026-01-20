use {
    crate::{
        commands::{Command, CommandFlow, NavigationTarget, navigation::NavigationSection},
        context::ScillaContext,
        misc::helpers::decode_and_deserialize_transaction,
        prompt::{prompt_confirmation, prompt_encoding_options, prompt_input_data},
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_account_decoder::UiAccount,
    solana_rpc_client_api::config::{RpcSimulateTransactionConfig, RpcTransactionConfig},
    solana_signature::Signature,
    solana_transaction_status::{
        EncodedTransaction, UiInnerInstructions, UiMessage, UiTransactionEncoding,
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
                show_spinner(self.spinner_msg(), fetch_transaction(ctx, &signature)).await;
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

                let relaxed = prompt_confirmation(
                    "Use relaxed mode (skip signature verification, refresh blockhash)? (y/n):",
                );

                let encoding = prompt_encoding_options();

                let encoded_tx: String = prompt_input_data("Enter encoded transaction:");

                show_spinner(
                    self.spinner_msg(),
                    simulate_transaction(ctx, encoding, &encoded_tx, relaxed),
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

async fn fetch_transaction(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()> {
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
    relaxed: bool,
) -> anyhow::Result<()> {
    let tx = decode_and_deserialize_transaction(encoding, encoded_tx)?;

    let response = match relaxed {
        true => {
            ctx.rpc()
                .simulate_transaction_with_config(
                    &tx,
                    RpcSimulateTransactionConfig {
                        // Be able to simulate with older transactions
                        // Guarantee a flexible simulation environment
                        replace_recent_blockhash: true,
                        sig_verify: false,
                        commitment: Some(ctx.rpc().commitment()),
                        ..Default::default()
                    },
                )
                .await?
        }
        false => {
            ctx.rpc()
                .simulate_transaction_with_config(
                    &tx,
                    RpcSimulateTransactionConfig {
                        commitment: Some(ctx.rpc().commitment()),
                        ..Default::default()
                    },
                )
                .await?
        }
    };
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

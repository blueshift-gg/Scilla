use {
    crate::{
        commands::CommandExec,
        constants::{ACTIVE_STAKE_EPOCH_BOUND, DEFAULT_EPOCH_LIMIT, STAKE_HISTORY_SYSVAR_ADDR},
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::{
            SolAmount, bincode_deserialize, bincode_deserialize_with_limit, build_and_send_tx,
            fetch_account_with_epoch, lamports_to_sol, read_keypair_from_path, sol_to_lamports,
        },
        prompt::prompt_data,
        ui::show_spinner,
    },
    anyhow::bail,
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_clock::Clock,
    solana_keypair::Signer,
    solana_pubkey::Pubkey,
    solana_sdk_ids::sysvar::stake_history,
    solana_stake_interface::{
        instruction::{self, deactivate_stake, merge, withdraw},
        program::id as stake_program_id,
        stake_history::{StakeHistory, StakeHistoryEntry},
        state::{Meta, StakeActivationStatus, StakeStateV2},
    },
    solana_sysvar::clock,
    std::{fmt, path::PathBuf},
};

/// Commands related to staking operations
#[derive(Debug, Clone)]
pub enum StakeCommand {
    Create,
    Delegate,
    Deactivate,
    Withdraw,
    Merge,
    Split,
    Show,
    History,
    GoBack,
}

impl StakeCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            StakeCommand::Create => "Creating new stake account…",
            StakeCommand::Delegate => "Delegating stake to validator…",
            StakeCommand::Deactivate => "Deactivating stake (cooldown starting)…",
            StakeCommand::Withdraw => "Withdrawing SOL from deactivated stake…",
            StakeCommand::Merge => "Merging stake accounts…",
            StakeCommand::Split => "Splitting stake into multiple accounts…",
            StakeCommand::Show => "Fetching stake account details…",
            StakeCommand::History => "Fetching stake account history…",
            StakeCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for StakeCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            StakeCommand::Create => "Create stake account",
            StakeCommand::Delegate => "Delegate stake",
            StakeCommand::Deactivate => "Deactivate stake",
            StakeCommand::Withdraw => "Withdraw stake",
            StakeCommand::Merge => "Merge stake accounts",
            StakeCommand::Split => "Split stake account",
            StakeCommand::Show => "Show stake",
            StakeCommand::History => "View stake history",
            StakeCommand::GoBack => "Go back",
        };
        write!(f, "{command}")
    }
}

impl StakeCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            StakeCommand::Create => todo!(),
            StakeCommand::Delegate => todo!(),
            StakeCommand::Deactivate => {
                let stake_pubkey: Pubkey =
                    prompt_data("Enter Stake Account Pubkey to Deactivate:")?;
                show_spinner(
                    self.spinner_msg(),
                    process_deactivate_stake_account(ctx, &stake_pubkey),
                )
                .await?;
            }
            StakeCommand::Withdraw => {
                let stake_pubkey: Pubkey =
                    prompt_data("Enter Stake Account Pubkey to Withdraw from:")?;
                let recipient: Pubkey = prompt_data("Enter Recipient Address:")?;
                let amount: SolAmount = prompt_data("Enter Amount to Withdraw (SOL):")?;

                show_spinner(
                    self.spinner_msg(),
                    process_withdraw_stake(ctx, &stake_pubkey, &recipient, amount.value()),
                )
                .await?;
            }
            StakeCommand::Merge => {
                let destination_stake_account_pubkey: Pubkey =
                    prompt_data("Enter Stake Account Pubkey: ")?;
                let source_stake_account_pubkey: Pubkey =
                    prompt_data("Enter Source Stake Account Pubkey: ")?;
                let stake_authority_keypair_path: PathBuf =
                    prompt_data("Enter Stake Authority Keypair Path: ")?;

                show_spinner(
                    self.spinner_msg(),
                    process_merge_stake(
                        ctx,
                        &destination_stake_account_pubkey,
                        &source_stake_account_pubkey,
                        &stake_authority_keypair_path,
                    ),
                )
                .await?;
            }
            StakeCommand::Split => {
                let stake_account_pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey: ")?;
                let split_stake_account_pubkey: Pubkey =
                    prompt_data("Enter Split Stake Account Pubkey: ")?;
                let stake_authority_keypair_path: PathBuf =
                    prompt_data("Enter Stake Authority Keypair Path: ")?;
                let amount_to_split: f64 = prompt_data("Enter Stake Amount (SOL) to Split: ")?;

                show_spinner(
                    self.spinner_msg(),
                    process_split_stake(
                        ctx,
                        &stake_account_pubkey,
                        &split_stake_account_pubkey,
                        &stake_authority_keypair_path,
                        amount_to_split,
                    ),
                )
                .await?;
            }
            StakeCommand::Show => {
                let pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey:")?;
                show_spinner(self.spinner_msg(), show_stake_account(ctx, &pubkey)).await?;
            }
            StakeCommand::History => {
                show_spinner(self.spinner_msg(), process_stake_history(ctx)).await?;
            }

            StakeCommand::GoBack => return Ok(CommandExec::GoBack),
        }

        Ok(CommandExec::Process(()))
    }
}

async fn show_stake_account(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let accounts = ctx
        .rpc()
        .get_multiple_accounts(&[*pubkey, stake_history::id(), clock::id()])
        .await?;

    let Some(Some(stake_account)) = accounts.first() else {
        anyhow::bail!("Failed to get stake account");
    };

    let Some(Some(stake_history_account)) = accounts.get(1) else {
        anyhow::bail!("Failed to get stake history account");
    };

    let Some(Some(clock_account)) = accounts.get(2) else {
        anyhow::bail!("Failed to get clock account");
    };

    let stake_history: StakeHistory =
        bincode_deserialize(&stake_history_account.data, "stake history account data")?;
    let clock: Clock = bincode_deserialize(&clock_account.data, "clock account data")?;

    let stake_state: StakeStateV2 = bincode_deserialize(&stake_account.data, "stake account data")?;

    let current_epoch = clock.epoch;

    // Build main table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Stake Account Pubkey"),
            Cell::new(pubkey.to_string()),
        ])
        .add_row(vec![
            Cell::new("Account Balance (SOL)"),
            Cell::new(lamports_to_sol(stake_account.lamports)),
        ])
        .add_row(vec![
            Cell::new("Account Balance (Lamports)"),
            Cell::new(stake_account.lamports.to_string()),
        ])
        .add_row(vec![
            Cell::new("Rent Epoch"),
            Cell::new(stake_account.rent_epoch.to_string()),
        ]);

    // Add stake state specific information
    match &stake_state {
        StakeStateV2::Uninitialized => {
            table.add_row(vec![Cell::new("Stake State"), Cell::new("Uninitialized")]);
        }
        StakeStateV2::Initialized(Meta {
            rent_exempt_reserve,
            authorized,
            lockup,
        }) => {
            table
                .add_row(vec![Cell::new("Stake State"), Cell::new("Initialized")])
                .add_row(vec![
                    Cell::new("Rent Exempt Reserve (Lamports)"),
                    Cell::new(rent_exempt_reserve.to_string()),
                ])
                .add_row(vec![
                    Cell::new("Stake Authority"),
                    Cell::new(authorized.staker.to_string()),
                ])
                .add_row(vec![
                    Cell::new("Withdraw Authority"),
                    Cell::new(authorized.withdrawer.to_string()),
                ]);

            if lockup.is_in_force(&clock, None) {
                table
                    .add_row(vec![
                        Cell::new("Lockup Epoch"),
                        Cell::new(lockup.epoch.to_string()),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Unix Timestamp"),
                        Cell::new(lockup.unix_timestamp.to_string()),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Custodian"),
                        Cell::new(lockup.custodian.to_string()),
                    ]);
            }
        }
        StakeStateV2::Stake(
            Meta {
                authorized, lockup, ..
            },
            stake,
            _,
        ) => {
            // Calculate activation status
            let StakeActivationStatus {
                effective,
                activating,
                deactivating,
            } = stake.delegation.stake_activating_and_deactivating(
                current_epoch,
                &stake_history,
                None,
            );

            table
                .add_row(vec![Cell::new("Stake State"), Cell::new("Delegated")])
                .add_row(vec![
                    Cell::new("Stake Authority"),
                    Cell::new(authorized.staker.to_string()),
                ])
                .add_row(vec![
                    Cell::new("Withdraw Authority"),
                    Cell::new(authorized.withdrawer.to_string()),
                ])
                .add_row(vec![
                    Cell::new("Delegated Vote Account"),
                    Cell::new(stake.delegation.voter_pubkey.to_string()),
                ])
                .add_row(vec![
                    Cell::new("Delegated Stake (SOL)"),
                    Cell::new(lamports_to_sol(stake.delegation.stake)),
                ])
                .add_row(vec![
                    Cell::new("Activation Epoch"),
                    Cell::new(if stake.delegation.activation_epoch < u64::MAX {
                        stake.delegation.activation_epoch.to_string()
                    } else {
                        "N/A".to_string()
                    }),
                ])
                .add_row(vec![
                    Cell::new("Deactivation Epoch"),
                    Cell::new(if stake.delegation.deactivation_epoch < u64::MAX {
                        stake.delegation.deactivation_epoch.to_string()
                    } else {
                        "N/A".to_string()
                    }),
                ])
                .add_row(vec![
                    Cell::new("Active Stake (SOL)"),
                    Cell::new(lamports_to_sol(effective)),
                ])
                .add_row(vec![
                    Cell::new("Activating Stake (SOL)"),
                    Cell::new(lamports_to_sol(activating)),
                ])
                .add_row(vec![
                    Cell::new("Deactivating Stake (SOL)"),
                    Cell::new(lamports_to_sol(deactivating)),
                ])
                .add_row(vec![
                    Cell::new("Credits Observed"),
                    Cell::new(stake.credits_observed.to_string()),
                ]);

            if lockup.is_in_force(&clock, None) {
                table
                    .add_row(vec![
                        Cell::new("Lockup Epoch"),
                        Cell::new(lockup.epoch.to_string()),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Unix Timestamp"),
                        Cell::new(lockup.unix_timestamp.to_string()),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Custodian"),
                        Cell::new(lockup.custodian.to_string()),
                    ]);
            }
        }
        StakeStateV2::RewardsPool => {
            table.add_row(vec![Cell::new("Stake State"), Cell::new("Rewards Pool")]);
        }
    }

    println!("{}\n", style("STAKE ACCOUNT INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

async fn process_deactivate_stake_account(
    ctx: &ScillaContext,
    stake_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let account = ctx.rpc().get_account(stake_pubkey).await?;

    if account.owner != stake_program_id() {
        bail!("Account is not owned by the stake program");
    }

    let stake_state: StakeStateV2 = bincode_deserialize(&account.data, "stake account data")?;

    match stake_state {
        StakeStateV2::Stake(meta, stake, _) => {
            if stake.delegation.deactivation_epoch != ACTIVE_STAKE_EPOCH_BOUND {
                bail!(
                    "Stake is already deactivating at epoch {}",
                    stake.delegation.deactivation_epoch
                );
            }

            if &meta.authorized.staker != ctx.pubkey() {
                bail!(
                    "You are not the authorized staker. Authorized staker: {}",
                    meta.authorized.staker
                );
            }
        }
        StakeStateV2::Initialized(_) => {
            bail!("Stake account is initialized but not delegated");
        }
        _ => {
            bail!("Stake account is not in a valid state for deactivation");
        }
    }

    let authorized_pubkey = ctx.pubkey();
    let instruction = deactivate_stake(stake_pubkey, authorized_pubkey);

    let signature = build_and_send_tx(ctx, &[instruction], &[ctx.keypair()]).await?;

    println!(
        "\n{} {}\n{}\n{}",
        style("Stake Deactivated Successfully!").green().bold(),
        style("(Cooldown will take 1-2 epochs ≈ 2-4 days)").yellow(),
        style(format!("Stake Account: {stake_pubkey}")).yellow(),
        style(format!("Signature: {signature}")).cyan()
    );

    Ok(())
}

async fn process_withdraw_stake(
    ctx: &ScillaContext,
    stake_pubkey: &Pubkey,
    recipient: &Pubkey,
    amount_sol: f64,
) -> anyhow::Result<()> {
    let amount_lamports = sol_to_lamports(amount_sol);

    let (account, epoch_info) = fetch_account_with_epoch(ctx, stake_pubkey).await?;

    if account.owner != stake_program_id() {
        bail!("Account is not owned by the stake program");
    }

    let stake_state: StakeStateV2 = bincode_deserialize(&account.data, "stake account data")?;

    match stake_state {
        StakeStateV2::Stake(meta, stake, _) => {
            if &meta.authorized.withdrawer != ctx.pubkey() {
                bail!(
                    "You are not the authorized withdrawer. Authorized withdrawer: {}",
                    meta.authorized.withdrawer
                );
            }

            if stake.delegation.deactivation_epoch == ACTIVE_STAKE_EPOCH_BOUND {
                bail!(
                    "Stake is still active. You must deactivate it first and wait for the \
                     cooldown period."
                );
            }

            if epoch_info.epoch <= stake.delegation.deactivation_epoch {
                let epochs_remaining = stake.delegation.deactivation_epoch - epoch_info.epoch;
                bail!(
                    "Stake is still cooling down. Current epoch: {}, deactivation epoch: {}, \
                     epochs remaining: {}",
                    epoch_info.epoch,
                    stake.delegation.deactivation_epoch,
                    epochs_remaining
                );
            }
        }
        StakeStateV2::Initialized(meta) => {
            if &meta.authorized.withdrawer != ctx.pubkey() {
                bail!(
                    "You are not the authorized withdrawer. Authorized withdrawer: {}",
                    meta.authorized.withdrawer
                );
            }
        }
        StakeStateV2::Uninitialized => {
            bail!("Stake account is uninitialized");
        }
        StakeStateV2::RewardsPool => {
            bail!("Cannot withdraw from rewards pool");
        }
    }

    if amount_lamports > account.lamports {
        bail!(
            "Insufficient balance. Have {:.6} SOL, trying to withdraw {:.6} SOL",
            lamports_to_sol(account.lamports),
            amount_sol
        );
    }

    let withdrawer_pubkey = ctx.pubkey();

    let instruction = withdraw(
        stake_pubkey,
        withdrawer_pubkey,
        recipient,
        amount_lamports,
        None,
    );

    let signature = build_and_send_tx(ctx, &[instruction], &[ctx.keypair()]).await?;

    println!(
        "\n{} {}\n{}\n{}\n{}",
        style("Stake Withdrawn Successfully!").green().bold(),
        style(format!("From Stake Account: {stake_pubkey}")).yellow(),
        style(format!("To Recipient: {recipient}")).yellow(),
        style(format!("Amount: {amount_sol} SOL")).cyan(),
        style(format!("Signature: {signature}")).cyan()
    );

    Ok(())
}

async fn process_merge_stake(
    ctx: &ScillaContext,
    destination_stake_account_pubkey: &Pubkey,
    source_stake_account_pubkey: &Pubkey,
    stake_authority_keypair_path: &PathBuf,
) -> anyhow::Result<()> {
    let stake_authority_keypair = read_keypair_from_path(stake_authority_keypair_path)?;

    // checks for unique pubkeys
    if destination_stake_account_pubkey == source_stake_account_pubkey {
        bail!(
            "Destination Stake Account {} & Source Stake Account {} must not be the same",
            destination_stake_account_pubkey,
            source_stake_account_pubkey
        );
    }

    let stake_accounts = ctx
        .rpc()
        .get_multiple_accounts(&[
            *destination_stake_account_pubkey,
            *source_stake_account_pubkey,
        ])
        .await?;

    let Some(destination_stake_account) = stake_accounts[0].as_ref() else {
        bail!("Failed to get stake account");
    };

    let Some(source_stake_account) = stake_accounts[1].as_ref() else {
        bail!("Failed to get stake account");
    };

    let destination_stake_state: StakeStateV2 = bincode_deserialize(
        &destination_stake_account.data,
        "destination stake account data",
    )?;

    let source_stake_state: StakeStateV2 =
        bincode_deserialize(&source_stake_account.data, "source stake account data")?;

    match &destination_stake_state {
        StakeStateV2::Initialized(_) => {
            // Initialized destination is valid
        }
        StakeStateV2::Stake(_, _, _) => {
            // Delegated destination is valid
        }
        _ => bail!("Destination stake account is not in a valid state"),
    }

    match &source_stake_state {
        StakeStateV2::Initialized(meta) => {
            // CHECK: Verify authority for initialized source
            if meta.authorized.staker != stake_authority_keypair.pubkey() {
                bail!(
                    "Provided keypair is not the stake authority for source account\nExpected: \
                     {}\nProvided: {}",
                    meta.authorized.staker,
                    stake_authority_keypair.pubkey()
                );
            }
        }
        StakeStateV2::Stake(meta, stake, _) => {
            // CHECK: Verify authority for delegated source
            if meta.authorized.staker != stake_authority_keypair.pubkey() {
                bail!(
                    "Provided keypair is not the stake authority for source account\nExpected: \
                     {}\nProvided: {}",
                    meta.authorized.staker,
                    stake_authority_keypair.pubkey()
                );
            }

            // CHECK: Source must not be deactivating
            if stake.delegation.deactivation_epoch != u64::MAX {
                bail!(
                    "Cannot merge: source stake account is deactivating at epoch {}",
                    stake.delegation.deactivation_epoch
                );
            }
        }
        _ => bail!("Source stake account is not in a valid state"),
    }

    let stake_authority_pubkey = stake_authority_keypair.pubkey();

    let ixs = merge(
        destination_stake_account_pubkey,
        source_stake_account_pubkey,
        &stake_authority_pubkey,
    );

    let signature =
        build_and_send_tx(ctx, &ixs, &[ctx.keypair(), &stake_authority_keypair]).await?;

    println!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        style("Stake Merged successfully!").yellow().bold(),
        style(format!(
            "Destination Stake Account: {}",
            destination_stake_account_pubkey
        ))
        .yellow(),
        style(format!(
            "Source Stake Account: {}",
            source_stake_account_pubkey
        ))
        .yellow(),
        style(format!("Stake Authority: {}", stake_authority_pubkey)).yellow(),
        style(format!(
            "After Merge: {} SOL",
            lamports_to_sol(destination_stake_account.lamports)
        ))
        .cyan(),
        style(format!("Signature: {}", signature)).green()
    );

    Ok(())
}

async fn process_split_stake(
    ctx: &ScillaContext,
    stake_account_pubkey: &Pubkey,
    split_stake_account_pubkey: &Pubkey,
    stake_authority_keypair_path: &PathBuf,
    amount_to_split: f64,
) -> anyhow::Result<()> {
    let stake_authority_keypair = read_keypair_from_path(stake_authority_keypair_path)?;
    let stake_authority_pubkey = stake_authority_keypair.pubkey();
    let lamports: u64 = sol_to_lamports(amount_to_split);

    if stake_account_pubkey == split_stake_account_pubkey {
        bail!(
            "Existing Stake Account {} and New Split Stake Account {} must not be the same",
            stake_account_pubkey,
            split_stake_account_pubkey
        );
    }

    let stake_minimum_delegation = ctx.rpc().get_stake_minimum_delegation().await?;

    if lamports < stake_minimum_delegation {
        bail!(
            "Need at least {} lamports for minimum stake delegation, but you provided {}",
            stake_minimum_delegation,
            lamports
        );
    }

    let ix = instruction::split(
        stake_account_pubkey,
        &stake_authority_pubkey,
        lamports,
        split_stake_account_pubkey,
    );

    let signature = build_and_send_tx(ctx, &ix, &[ctx.keypair(), &stake_authority_keypair]).await?;

    println!(
        "{}\n{}\n{}\n{}\n{}",
        style("Split Stake successfully!").yellow().bold(),
        style(format!("Stake Account: {}", stake_account_pubkey)).yellow(),
        style(format!(
            "Split Stake Account: {}",
            split_stake_account_pubkey
        ))
        .yellow(),
        style(format!("Stake Authority: {}", stake_authority_pubkey)).yellow(),
        style(format!("Signature: {}", signature)).green()
    );

    Ok(())
}

async fn process_stake_history(ctx: &ScillaContext) -> anyhow::Result<()> {
    let stake_history_sysvar = Pubkey::from_str_const(STAKE_HISTORY_SYSVAR_ADDR);

    let account = ctx.rpc().get_account(&stake_history_sysvar).await?;

    let stake_history: StakeHistory =
        bincode_deserialize_with_limit(account.data.len() as u64, &account.data, "stake history")?;

    if stake_history.is_empty() {
        println!("\n{}", style("No stake history available").yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Epoch").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Effective Stake").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Activating Stake").add_attribute(comfy_table::Attribute::Bold),
        Cell::new("Deactivating Stake").add_attribute(comfy_table::Attribute::Bold),
    ]);

    for (epoch, entry) in stake_history.iter().take(DEFAULT_EPOCH_LIMIT) {
        let StakeHistoryEntry {
            effective,
            activating,
            deactivating,
        } = entry;

        table.add_row(vec![
            Cell::new(epoch.to_string()),
            Cell::new(lamports_to_sol(*effective)),
            Cell::new(lamports_to_sol(*activating)),
            Cell::new(lamports_to_sol(*deactivating)),
        ]);
    }

    println!("\n{}", style("CLUSTER STAKE HISTORY").green().bold());
    println!("{}", table);

    Ok(())
}

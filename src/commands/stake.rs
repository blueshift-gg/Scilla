use {
    crate::{
        commands::CommandExec,
        constants::{ACTIVE_STAKE_EPOCH_BOUND, DEFAULT_EPOCH_LIMIT, STAKE_HISTORY_SYSVAR_ADDR},
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::{
            SolAmount, build_and_send_tx, fetch_account_with_epoch, lamports_to_sol,
            sol_to_lamports,
        },
        prompt::prompt_data,
        ui::show_spinner,
    },
    anyhow::bail,
    bincode::Options,
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_pubkey::Pubkey,
    solana_stake_interface::{
        instruction::{deactivate_stake, withdraw},
        program::id as stake_program_id,
        stake_history::{StakeHistory, StakeHistoryEntry},
        state::StakeStateV2,
    },
    std::fmt,
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
            StakeCommand::Merge => todo!(),
            StakeCommand::Split => todo!(),
            StakeCommand::Show => todo!(),
            StakeCommand::History => {
                show_spinner(self.spinner_msg(), process_stake_history(ctx)).await?;
            }

            StakeCommand::GoBack => return Ok(CommandExec::GoBack),
        }

        Ok(CommandExec::Process(()))
    }
}

async fn process_deactivate_stake_account(
    ctx: &ScillaContext,
    stake_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let account = ctx.rpc().get_account(stake_pubkey).await?;

    if account.owner != stake_program_id() {
        bail!("Account is not owned by the stake program");
    }

    let stake_state: StakeStateV2 = bincode::deserialize(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize stake account: {e}"))?;

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

    let stake_state: StakeStateV2 = bincode::deserialize(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize stake account: {e}"))?;

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

async fn process_stake_history(ctx: &ScillaContext) -> anyhow::Result<()> {
    let stake_history_sysvar = Pubkey::from_str_const(STAKE_HISTORY_SYSVAR_ADDR);

    let account = ctx.rpc().get_account(&stake_history_sysvar).await?;

    let stake_history: StakeHistory = bincode::options()
        .with_fixint_encoding()
        .with_limit(account.data.len() as u64)
        .deserialize(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize StakeHistory: {}", e))?;

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

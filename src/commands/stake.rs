use {
    crate::{
        commands::CommandExec,
        constants::ACTIVE_STAKE_EPOCH_BOUND,
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::{build_and_send_tx, lamports_to_sol, sol_to_lamports, SolAmount},
        prompt::prompt_data,
        ui::show_spinner,
    },
    anyhow::bail,
    console::style,
    solana_pubkey::Pubkey,
    solana_stake_interface::{
        instruction::{deactivate_stake, withdraw},
        program::id as stake_program_id,
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
            StakeCommand::Create => "Create",
            StakeCommand::Delegate => "Delegate",
            StakeCommand::Deactivate => "Deactivate",
            StakeCommand::Withdraw => "Withdraw",
            StakeCommand::Merge => "Merge",
            StakeCommand::Split => "Split",
            StakeCommand::Show => "Show",
            StakeCommand::History => "History",
            StakeCommand::GoBack => "Go Back",
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
            StakeCommand::History => todo!(),
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

    let account = ctx.rpc().get_account(stake_pubkey).await?;

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

            let epoch_info = ctx.rpc().get_epoch_info().await?;
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

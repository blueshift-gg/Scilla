use {
    crate::{
        commands::CommandExec, constants::LAMPORTS_PER_SOL, context::ScillaContext,
        error::ScillaResult, prompt::prompt_data, ui::show_spinner,
    },
    bincode,
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_clock::Clock,
    solana_pubkey::Pubkey,
    solana_sdk_ids::sysvar::stake_history,
    solana_stake_interface::{
        stake_history::StakeHistory,
        state::{Meta, Stake, StakeActivationStatus, StakeStateV2},
    },
    solana_sysvar::clock,
    std::{ops::Div, str::FromStr},
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
    pub fn description(&self) -> &'static str {
        match self {
            StakeCommand::Create => "Create a new stake account",
            StakeCommand::Delegate => "Delegate stake to a validator",
            StakeCommand::Deactivate => "Begin stake cooldown",
            StakeCommand::Withdraw => "Withdraw SOL from deactivated stake",
            StakeCommand::Merge => "Combine two stake accounts",
            StakeCommand::Split => "Split stake into multiple accounts",
            StakeCommand::Show => "Display stake account details",
            StakeCommand::History => "View stake account history",
            StakeCommand::GoBack => "Go back",
        }
    }
}

impl StakeCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            StakeCommand::Create => todo!(),
            StakeCommand::Delegate => todo!(),
            StakeCommand::Deactivate => todo!(),
            StakeCommand::Withdraw => todo!(),
            StakeCommand::Merge => todo!(),
            StakeCommand::Split => todo!(),
            StakeCommand::Show => {
                let pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey:")?;
                show_spinner(self.description(), show_stake_account(ctx, &pubkey)).await?;
            }
            StakeCommand::History => todo!(),
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

    let stake_account = match accounts.get(0) {
        Some(account) => match account {
            Some(data) => data,
            None => return Err(anyhow::anyhow!("Failed to get stake account data")),
        },
        None => return Err(anyhow::anyhow!("Failed to get stake account")),
    };

    let stake_history_account = match accounts.get(1) {
        Some(account) => match account {
            Some(data) => data,
            None => return Err(anyhow::anyhow!("Failed to get stake history account data")),
        },
        None => return Err(anyhow::anyhow!("Failed to get stake history account")),
    };
    let clock_account = match accounts.get(2) {
        Some(account) => match account {
            Some(data) => data,
            None => return Err(anyhow::anyhow!("Failed to get clock account data")),
        },
        None => return Err(anyhow::anyhow!("Failed to get clock account")),
    };

    let stake_history: StakeHistory = bincode::deserialize(&stake_history_account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize stake history: {}", e))?;
    let clock: Clock = bincode::deserialize(&clock_account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize clock: {}", e))?;

    let stake_state: StakeStateV2 = bincode::deserialize(&stake_account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize stake state: {}", e))?;

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
            Cell::new(stake_state.stake().unwrap_or_default().delegation.stake),
        ])
        .add_row(vec![
            Cell::new("Account Balance (Lamports)"),
            Cell::new(format!("{}", stake_account.lamports)),
        ])
        .add_row(vec![
            Cell::new("Rent Epoch"),
            Cell::new(format!("{}", stake_account.rent_epoch)),
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
                    Cell::new(format!("{:.9}", rent_exempt_reserve)),
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
                        Cell::new(format!("{}", lockup.epoch)),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Unix Timestamp"),
                        Cell::new(format!("{}", lockup.unix_timestamp)),
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
                    Cell::new(format!(
                        "{:.9}",
                        (stake.delegation.stake as f64).div(LAMPORTS_PER_SOL as f64)
                    )),
                ])
                .add_row(vec![
                    Cell::new("Activation Epoch"),
                    Cell::new(if stake.delegation.activation_epoch < u64::MAX {
                        format!("{}", stake.delegation.activation_epoch)
                    } else {
                        "N/A".to_string()
                    }),
                ])
                .add_row(vec![
                    Cell::new("Deactivation Epoch"),
                    Cell::new(if stake.delegation.deactivation_epoch < u64::MAX {
                        format!("{}", stake.delegation.deactivation_epoch)
                    } else {
                        "N/A".to_string()
                    }),
                ])
                .add_row(vec![
                    Cell::new("Active Stake (SOL)"),
                    Cell::new(format!(
                        "{:.9}",
                        (effective as f64).div(LAMPORTS_PER_SOL as f64)
                    )),
                ])
                .add_row(vec![
                    Cell::new("Activating Stake (SOL)"),
                    Cell::new(format!(
                        "{:.9}",
                        (activating as f64).div(LAMPORTS_PER_SOL as f64)
                    )),
                ])
                .add_row(vec![
                    Cell::new("Deactivating Stake (SOL)"),
                    Cell::new(format!(
                        "{:.9}",
                        (deactivating as f64).div(LAMPORTS_PER_SOL as f64)
                    )),
                ])
                .add_row(vec![
                    Cell::new("Credits Observed"),
                    Cell::new(format!("{}", stake.credits_observed)),
                ]);

            if lockup.is_in_force(&clock, None) {
                table
                    .add_row(vec![
                        Cell::new("Lockup Epoch"),
                        Cell::new(format!("{}", lockup.epoch)),
                    ])
                    .add_row(vec![
                        Cell::new("Lockup Unix Timestamp"),
                        Cell::new(format!("{}", lockup.unix_timestamp)),
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

    println!("\n{}", style("STAKE ACCOUNT INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

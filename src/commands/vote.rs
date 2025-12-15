use anyhow::anyhow;
use solana_keypair::{EncodableKey, Keypair, Signer};
use solana_pubkey::Pubkey;
use solana_sdk::{message::Message, transaction::Transaction};
use solana_vote_program::{
    vote_instruction::{self, CreateVoteAccountConfig, withdraw},
    vote_state::{VoteAuthorize, VoteInit, VoteStateV4},
};
use std::path::PathBuf;
use {
    crate::{
        ScillaContext, ScillaResult, commands::CommandExec, prompt::prompt_data, ui::show_spinner,
    },
    ::console::style,
};

/// Commands related to validator/vote account operations
#[derive(Debug, Clone)]
pub enum VoteCommand {
    CreateVoteAccount,
    AuthorizeVoter,
    WithdrawFromVoteAccount,
    ShowVoteAccount,
    GoBack,
}

impl VoteCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            VoteCommand::CreateVoteAccount => "Creating vote account…",
            VoteCommand::AuthorizeVoter => "Authorizing voter…",
            VoteCommand::WithdrawFromVoteAccount => "Withdrawing SOL from vote account…",
            VoteCommand::ShowVoteAccount => "Fetching vote account details…",
            VoteCommand::GoBack => "Going back…",
        }
    }
}

impl VoteCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            VoteCommand::ShowVoteAccount => {
                let pubkey: Pubkey = prompt_data("Enter Vote Account Pubkey:")?;
                show_spinner(self.spinner_msg(), show_vote_account(ctx, &pubkey)).await?;
            }
            VoteCommand::CreateVoteAccount => todo!(),
            VoteCommand::AuthorizeVoter => todo!(),
            VoteCommand::WithdrawFromVoteAccount => todo!(),
            VoteCommand::GoBack => return Ok(CommandExec::GoBack),
        }
        Ok(CommandExec::Process(()))
    }
}

async fn show_vote_account(ctx: &ScillaContext, pubkey: &Pubkey) -> anyhow::Result<()> {
    let vote_accounts = ctx.rpc().get_vote_accounts().await?;

    let vote_account = vote_accounts
        .current
        .iter()
        .find(|va| va.vote_pubkey == pubkey.to_string())
        .or_else(|| {
            vote_accounts
                .delinquent
                .iter()
                .find(|va| va.vote_pubkey == pubkey.to_string())
        });

    match vote_account {
        Some(va) => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_header(vec![
                    Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
                ])
                .add_row(vec![
                    Cell::new("Vote Account"),
                    Cell::new(va.vote_pubkey.clone()),
                ])
                .add_row(vec![
                    Cell::new("Node Pubkey"),
                    Cell::new(va.node_pubkey.clone()),
                ])
                .add_row(vec![
                    Cell::new("Commission"),
                    Cell::new(format!("{}%", va.commission)),
                ])
                .add_row(vec![
                    Cell::new("Activated Stake (SOL)"),
                    Cell::new(format!(
                        "{:.2}",
                        va.activated_stake as f64 / 1_000_000_000.0
                    )),
                ])
                .add_row(vec![
                    Cell::new("Last Vote"),
                    Cell::new(format!("{}", va.last_vote)),
                ])
                .add_row(vec![
                    Cell::new("Root Slot"),
                    Cell::new(format!("{}", va.root_slot)),
                ])
                .add_row(vec![
                    Cell::new("Status"),
                    Cell::new(
                        if vote_accounts
                            .current
                            .iter()
                            .any(|v| v.vote_pubkey == pubkey.to_string())
                        {
                            "Current"
                        } else {
                            "Delinquent"
                        },
                    ),
                ]);

            println!("\n{}", style("VOTE ACCOUNT INFO").green().bold());
            println!("{}", table);
        }
        None => {
            println!(
                "{} Vote account {} not found in current or delinquent validators.",
                style("⚠").yellow(),
                style(pubkey).cyan()
            );
impl VoteCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            VoteCommand::CreateVoteAccount => {
                let account_keypair_path: PathBuf = prompt_data("Enter Account Keypair:")?;
                let identity_keypair_path: PathBuf = prompt_data("Enter Identity Keypair:")?;
                let withdraw_keypair_path: PathBuf = prompt_data("Enter Withdraw Keypair:")?;

                let account_keypair =
                    Keypair::read_from_file(&account_keypair_path).map_err(|e| {
                        anyhow!(
                            "Failed to read keypair from {:?}, {}",
                            account_keypair_path,
                            e
                        )
                    })?;

                let identity_keypair =
                    Keypair::read_from_file(&identity_keypair_path).map_err(|e| {
                        anyhow!(
                            "Failed to read keypair from {:?}, {}",
                            identity_keypair_path,
                            e
                        )
                    })?;

                let withdraw_keypair =
                    Keypair::read_from_file(&withdraw_keypair_path).map_err(|e| {
                        anyhow!(
                            "Failed to read keypair from {:?}, {}",
                            withdraw_keypair_path,
                            e
                        )
                    })?;

                show_spinner(
                    self.description(),
                    create_vote_account(
                        ctx,
                        &account_keypair,
                        &identity_keypair,
                        &withdraw_keypair,
                    ),
                )
                .await?;
            }
            VoteCommand::AuthorizeVoter => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                let authorized_keypair_path: PathBuf = prompt_data("Enter Authorized Keypair:")?;
                let new_authorized_pubkey: Pubkey = prompt_data("Enter New Authorized Address:")?;

                let authorized_keypair = Keypair::read_from_file(&authorized_keypair_path)
                    .map_err(|e| {
                        anyhow!(
                            "Failed to read keypair from {:?}, {}",
                            authorized_keypair_path,
                            e
                        )
                    })?;

                show_spinner(
                    self.description(),
                    process_vote_authorize(
                        ctx,
                        &vote_account_pubkey,
                        &authorized_keypair,
                        &new_authorized_pubkey,
                    ),
                )
                .await?;
            }
            VoteCommand::WithdrawFromVote => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                let authorized_keypair_path: PathBuf =
                    prompt_data("Enter Authorized Withdraw Keypair:")?;
                let recipient_address: Pubkey = prompt_data("Enter Recipient Address:")?;

                let amount_str: String =
                    prompt_data("Enter withdraw amount in SOL (empty for max):")?;
                let amount: u64 = if amount_str.trim().is_empty() {
                    0
                } else {
                    let sol: f64 = amount_str.parse().map_err(|_| anyhow!("Invalid amount"))?;
                    (sol * 1_000_000_000.0) as u64
                };

                let authorized_keypair = Keypair::read_from_file(&authorized_keypair_path)
                    .map_err(|e| {
                        anyhow!(
                            "Failed to read keypair from {:?}, {}",
                            authorized_keypair_path,
                            e
                        )
                    })?;

                show_spinner(
                    self.description(),
                    process_sol_withdraw_from_vote_account(
                        ctx,
                        &vote_account_pubkey,
                        &authorized_keypair,
                        &recipient_address,
                        amount,
                    ),
                )
                .await?;
            }
            VoteCommand::ShowVoteAccount => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                show_spinner(
                    self.description(),
                    get_vote_account(ctx, &vote_account_pubkey),
                )
                .await?;
            }
            VoteCommand::GoBack => {
                return Ok(CommandExec::GoBack);
            }
        }

        Ok(CommandExec::Process(()))
    }

    Ok(())
}

async fn create_vote_account(
    ctx: &ScillaContext,
    vote_account_keypair: &Keypair,
    identity_keypair: &Keypair,
    authorized_withdrawer: &Keypair,
) -> anyhow::Result<()> {
    let vote_account_pubkey = vote_account_keypair.pubkey();
    let identity_pubkey = identity_keypair.pubkey();
    let withdrawer_pubkey = authorized_withdrawer.pubkey();
    let fee_payer_pubkey = ctx.pubkey();

    if fee_payer_pubkey == &vote_account_pubkey {
        return Err(anyhow!(
            "Fee payer {} cannot be the same as vote account {}",
            fee_payer_pubkey,
            vote_account_pubkey
        ));
    }
    if vote_account_pubkey == identity_pubkey {
        return Err(anyhow!(
            "Vote account {} cannot be the same as identity {}",
            vote_account_pubkey,
            identity_pubkey
        ));
    }

    // checking if vote account already exists
    if let Ok(response) = ctx.rpc().get_account(&vote_account_pubkey).await {
        let err_msg = if response.owner == solana_vote_program::id() {
            format!("Vote account {} already exists", vote_account_pubkey)
        } else {
            format!(
                "Account {} already exists and is not a vote account",
                vote_account_pubkey
            )
        };
        return Err(anyhow!(err_msg));
    }

    // min rent check
    let required_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(VoteStateV4::size_of())
        .await?
        .max(1);

    let fee_payer_balance = ctx.rpc().get_balance(fee_payer_pubkey).await?;
    if fee_payer_balance < required_balance {
        return Err(anyhow!(
            "Insufficient balance. Fee payer has {} lamports, need at least {} lamports (~{:.4} SOL)",
            fee_payer_balance,
            required_balance,
            required_balance as f64 / 1_000_000_000.0
        ));
    }

    let vote_init = VoteInit {
        node_pubkey: identity_pubkey,
        authorized_voter: identity_pubkey, // defaults to identity
        authorized_withdrawer: withdrawer_pubkey,
        commission: 0, // TODO: prompt for this
    };

    let instructions = vote_instruction::create_account_with_config(
        fee_payer_pubkey,
        &vote_account_pubkey,
        &vote_init,
        required_balance,
        CreateVoteAccountConfig::default(),
    );

    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    let message = Message::new(&instructions, Some(fee_payer_pubkey));
    let mut tx = Transaction::new_unsigned(message);

    let signers: Vec<&dyn Signer> = vec![ctx.keypair(), vote_account_keypair, identity_keypair];

    tx.try_sign(&signers, recent_blockhash)?;

    let signature = ctx.rpc().send_and_confirm_transaction(&tx).await?;

    println!(
        "{} {}",
        style("Vote account created successfully!").green().bold(),
        style(format!("Signature: {signature}")).cyan()
    );
    println!(
        "{} {}",
        style("Vote account address:").green(),
        style(vote_account_pubkey).cyan()
    );

    Ok(())
}

async fn process_vote_authorize(
    ctx: &ScillaContext,
    vote_account_pubkey: &Pubkey,
    authorized_keypair: &Keypair,
    new_authorized_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let fee_payer_pubkey = ctx.pubkey();
    let authorized_pubkey = authorized_keypair.pubkey();

    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{} account does not exist", vote_account_pubkey))?;

    if vote_account.owner != solana_vote_program::id() {
        return Err(anyhow!("{} is not a vote account", vote_account_pubkey));
    }

    let vote_state = VoteStateV4::deserialize(&vote_account.data, vote_account_pubkey)
        .map_err(|_| anyhow!("Account data could not be deserialized to vote state"))?;

    let current_epoch = ctx.rpc().get_epoch_info().await?.epoch;

    let current_authorized_voter = vote_state
        .authorized_voters
        .get_authorized_voter(current_epoch)
        .ok_or_else(|| anyhow!("Invalid vote account state; no authorized voters found"))?;

    if authorized_pubkey != current_authorized_voter
        && authorized_pubkey != vote_state.authorized_withdrawer
    {
        return Err(anyhow!(
            "Keypair {} is not the current authorized voter ({}) or withdrawer ({})",
            authorized_pubkey,
            current_authorized_voter,
            vote_state.authorized_withdrawer
        ));
    }

    let vote_ix = vote_instruction::authorize(
        vote_account_pubkey,
        &authorized_pubkey,
        new_authorized_pubkey,
        VoteAuthorize::Voter,
    );

    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;

    let message = Message::new(&[vote_ix], Some(fee_payer_pubkey));

    let mut tx = Transaction::new_unsigned(message);

    let signers: Vec<&dyn Signer> = vec![ctx.keypair(), authorized_keypair];

    tx.try_sign(&signers, recent_blockhash)?;

    let signature = ctx.rpc().send_and_confirm_transaction(&tx).await?;

    println!(
        "{} {}",
        style("Signature:").green().bold(),
        style(signature).cyan()
    );

    Ok(())
}

async fn process_sol_withdraw_from_vote_account(
    ctx: &ScillaContext,
    vote_account_pubkey: &Pubkey,
    authorized_withdrawer: &Keypair,
    recipient_address: &Pubkey,
    amount: u64,
) -> anyhow::Result<()> {
    let fee_payer_pubkey = ctx.pubkey();
    let withdrawer_pubkey = authorized_withdrawer.pubkey();

    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{} account does not exist", vote_account_pubkey))?;

    if vote_account.owner != solana_vote_program::id() {
        return Err(anyhow!("{} is not a vote account", vote_account_pubkey));
    }

    let vote_state = VoteStateV4::deserialize(&vote_account.data, vote_account_pubkey)
        .map_err(|_| anyhow!("Account data could not be deserialized to vote state"))?;

    if withdrawer_pubkey != vote_state.authorized_withdrawer {
        return Err(anyhow!(
            "Keypair {} is not the authorized withdrawer ({})",
            withdrawer_pubkey,
            vote_state.authorized_withdrawer
        ));
    }

    let current_balance = ctx.rpc().get_balance(vote_account_pubkey).await?;
    let minimum_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(VoteStateV4::size_of())
        .await?;

    let withdraw_amount = if amount == 0 {
        current_balance.saturating_sub(minimum_balance)
    } else {
        amount
    };

    let balance_remaining = current_balance.saturating_sub(withdraw_amount);

    if balance_remaining < minimum_balance && balance_remaining != 0 {
        return Err(anyhow!(
            "Withdraw amount too large. The vote account balance must be at least {:.9} SOL to remain rent exempt, or withdraw everything",
            minimum_balance as f64 / 1_000_000_000.0
        ));
    }

    if withdraw_amount == 0 {
        return Err(anyhow!("Nothing to withdraw"));
    }

    let withdraw_ix = withdraw(
        vote_account_pubkey,
        &withdrawer_pubkey,
        withdraw_amount,
        recipient_address,
    );

    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;

    let message = Message::new(&[withdraw_ix], Some(fee_payer_pubkey));

    let mut tx = Transaction::new_unsigned(message);

    let signers: Vec<&dyn Signer> = vec![ctx.keypair(), authorized_withdrawer];

    tx.try_sign(&signers, recent_blockhash)?;

    let signature = ctx.rpc().send_and_confirm_transaction(&tx).await?;

    println!(
        "{} {}",
        style("Signature:").green().bold(),
        style(signature).cyan()
    );

    Ok(())
}

async fn get_vote_account(ctx: &ScillaContext, vote_account_pubkey: &Pubkey) -> anyhow::Result<()> {
    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{} account does not exist", vote_account_pubkey))?;

    if vote_account.owner != solana_vote_program::id() {
        return Err(anyhow!("{} is not a vote account", vote_account_pubkey));
    }

    let vote_state = VoteStateV4::deserialize(&vote_account.data, vote_account_pubkey)
        .map_err(|_| anyhow!("Account data could not be deserialized to vote state"))?;

    let balance_sol = vote_account.lamports as f64 / 1_000_000_000.0;

    let root_slot = match vote_state.root_slot {
        Some(slot) => slot.to_string(),
        None => "~".to_string(),
    };

    let timestamp = chrono::DateTime::from_timestamp(vote_state.last_timestamp.timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    println!(
        "{} {} SOL",
        style("Account Balance:").green().bold(),
        balance_sol
    );
    println!(
        "{} {}",
        style("Validator Identity:").green().bold(),
        vote_state.node_pubkey
    );
    println!(
        "{} {}",
        style("Vote Authority:").green().bold(),
        vote_state
            .authorized_voters
            .last()
            .map(|(_, v)| v)
            .unwrap_or(&vote_state.node_pubkey)
    );
    println!(
        "{} {}",
        style("Withdraw Authority:").green().bold(),
        vote_state.authorized_withdrawer
    );
    println!(
        "{} {}",
        style("Credits:").green().bold(),
        vote_state.credits()
    );
    println!(
        "{} {}%",
        style("Commission:").green().bold(),
        vote_state.inflation_rewards_commission_bps / 100
    );
    println!("{} {}", style("Root Slot:").green().bold(), root_slot);
    println!(
        "{} {} from slot {}",
        style("Recent Timestamp:").green().bold(),
        timestamp,
        vote_state.last_timestamp.slot
    );

    Ok(())
}

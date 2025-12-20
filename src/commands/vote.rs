use {
    crate::{
        commands::CommandExec,
        misc::helpers::{
            build_and_send_tx, lamports_to_sol, read_keypair_from_path, Commission, SolAmount,
        },
        prompt::prompt_data,
        ui::show_spinner,
        ScillaContext, ScillaResult,
    },
    ::console::style,
    anyhow::{anyhow, bail},
    comfy_table::{presets::UTF8_FULL, Cell, Table},
    solana_keypair::{Keypair, Signer},
    solana_pubkey::Pubkey,
    solana_vote_program::{
        vote_instruction::{self, withdraw, CreateVoteAccountConfig},
        vote_state::{VoteAuthorize, VoteInit, VoteStateV4},
    },
    std::{fmt, path::PathBuf},
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

impl fmt::Display for VoteCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            VoteCommand::CreateVoteAccount => "Create Vote Account",
            VoteCommand::AuthorizeVoter => "Authorize Voter",
            VoteCommand::WithdrawFromVoteAccount => "Withdraw From Vote Account",
            VoteCommand::ShowVoteAccount => "Show Vote Account",
            VoteCommand::GoBack => "Go Back",
        };
        write!(f, "{text}")
    }
}

impl VoteCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            VoteCommand::CreateVoteAccount => {
                let account_keypair_path: PathBuf = prompt_data("Enter Account Keypair Path:")?;
                let identity_keypair_path: PathBuf = prompt_data("Enter Identity Keypair Path:")?;
                let withdraw_keypair_path: PathBuf = prompt_data("Enter Withdraw Keypair Path:")?;
                let commission: Commission = prompt_data("Enter Commission 0-100 (default 0):")?;

                let account_keypair = read_keypair_from_path(&account_keypair_path)?;

                let identity_keypair = read_keypair_from_path(&identity_keypair_path)?;

                let withdraw_keypair = read_keypair_from_path(&withdraw_keypair_path)?;

                show_spinner(
                    self.spinner_msg(),
                    process_create_vote_account(
                        ctx,
                        &account_keypair,
                        &identity_keypair,
                        &withdraw_keypair,
                        commission.value(),
                    ),
                )
                .await?;
            }
            VoteCommand::AuthorizeVoter => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                let authorized_keypair_path: PathBuf =
                    prompt_data("Enter Authorized Keypair Path:")?;
                let new_authorized_pubkey: Pubkey = prompt_data("Enter New Authorized Address:")?;

                let authorized_keypair = read_keypair_from_path(&authorized_keypair_path)?;

                show_spinner(
                    self.spinner_msg(),
                    process_authorize_voter(
                        ctx,
                        &vote_account_pubkey,
                        &authorized_keypair,
                        &new_authorized_pubkey,
                    ),
                )
                .await?;
            }
            VoteCommand::WithdrawFromVoteAccount => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                let authorized_keypair_path: PathBuf =
                    prompt_data("Enter Authorized Withdraw Keypair Path:")?;
                let recipient_address: Pubkey = prompt_data("Enter Recipient Address:")?;

                let amount: SolAmount = prompt_data("Enter withdraw amount in SOL:")?;
                let authorized_keypair = read_keypair_from_path(&authorized_keypair_path)?;

                show_spinner(
                    self.spinner_msg(),
                    process_sol_withdraw_from_vote_account(
                        ctx,
                        &vote_account_pubkey,
                        &authorized_keypair,
                        &recipient_address,
                        amount.to_lamports(),
                    ),
                )
                .await?;
            }
            VoteCommand::ShowVoteAccount => {
                let vote_account_pubkey: Pubkey = prompt_data("Enter Vote Account Address:")?;
                show_spinner(
                    self.spinner_msg(),
                    process_fetch_vote_account(ctx, &vote_account_pubkey),
                )
                .await?;
            }
            VoteCommand::GoBack => return Ok(CommandExec::GoBack),
        }

        Ok(CommandExec::Process(()))
    }
}

async fn process_create_vote_account(
    ctx: &ScillaContext,
    vote_account_keypair: &Keypair,
    identity_keypair: &Keypair,
    authorized_withdrawer: &Keypair,
    commission: u8,
) -> anyhow::Result<()> {
    let vote_account_pubkey = vote_account_keypair.pubkey();
    let identity_pubkey = identity_keypair.pubkey();
    let withdrawer_pubkey = authorized_withdrawer.pubkey();
    let fee_payer_pubkey = ctx.pubkey();

    if fee_payer_pubkey == &vote_account_pubkey {
        bail!(
            "Fee payer {fee_payer_pubkey} cannot be the same as vote account {vote_account_pubkey}"
        );
    }
    if vote_account_pubkey == identity_pubkey {
        bail!(
            "Vote account {vote_account_pubkey} cannot be the same as identity {identity_pubkey}"
        );
    }

    // checking if vote account already exists
    if let Ok(response) = ctx.rpc().get_account(&vote_account_pubkey).await {
        let err_msg = if response.owner == solana_vote_program::id() {
            format!("Vote account {vote_account_pubkey} already exists")
        } else {
            format!("Account {vote_account_pubkey} already exists and is not a vote account")
        };
        bail!(err_msg)
    }

    let required_balance = ctx
        .rpc()
        .get_minimum_balance_for_rent_exemption(VoteStateV4::size_of())
        .await?
        .max(1);

    let vote_init = VoteInit {
        node_pubkey: identity_pubkey,
        authorized_voter: identity_pubkey, // defaults to identity
        authorized_withdrawer: withdrawer_pubkey,
        commission,
    };

    let instructions = vote_instruction::create_account_with_config(
        fee_payer_pubkey,
        &vote_account_pubkey,
        &vote_init,
        required_balance,
        CreateVoteAccountConfig::default(),
    );

    let signature = build_and_send_tx(
        ctx,
        &instructions,
        &[ctx.keypair(), vote_account_keypair, identity_keypair],
    )
    .await?;

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

async fn process_authorize_voter(
    ctx: &ScillaContext,
    vote_account_pubkey: &Pubkey,
    authorized_keypair: &Keypair,
    new_authorized_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let authorized_pubkey = authorized_keypair.pubkey();

    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{vote_account_pubkey} account does not exist"))?;

    if vote_account.owner != solana_vote_program::id() {
        bail!("{vote_account_pubkey} is not a vote account");
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
        bail!(
            "Keypair {} is not the current authorized voter ({}) or withdrawer ({})",
            authorized_pubkey,
            current_authorized_voter,
            vote_state.authorized_withdrawer
        );
    }

    let vote_ix = vote_instruction::authorize(
        vote_account_pubkey,
        &authorized_pubkey,
        new_authorized_pubkey,
        VoteAuthorize::Voter,
    );

    let signature =
        build_and_send_tx(ctx, &[vote_ix], &[ctx.keypair(), authorized_keypair]).await?;

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
    let withdrawer_pubkey = authorized_withdrawer.pubkey();

    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{vote_account_pubkey} account does not exist"))?;

    if vote_account.owner != solana_vote_program::id() {
        bail!("{vote_account_pubkey} is not a vote account");
    }

    let vote_state = VoteStateV4::deserialize(&vote_account.data, vote_account_pubkey)
        .map_err(|_| anyhow!("Account data could not be deserialized to vote state"))?;

    if withdrawer_pubkey != vote_state.authorized_withdrawer {
        bail!(
            "Keypair {} is not the authorized withdrawer ({})",
            withdrawer_pubkey,
            vote_state.authorized_withdrawer
        );
    }

    let withdraw_ix = withdraw(
        vote_account_pubkey,
        &withdrawer_pubkey,
        amount,
        recipient_address,
    );

    let signature =
        build_and_send_tx(ctx, &[withdraw_ix], &[ctx.keypair(), authorized_withdrawer]).await?;

    println!(
        "{} {}",
        style("Signature:").green().bold(),
        style(signature).cyan()
    );

    Ok(())
}

async fn process_fetch_vote_account(
    ctx: &ScillaContext,
    vote_account_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let vote_account = ctx
        .rpc()
        .get_account(vote_account_pubkey)
        .await
        .map_err(|_| anyhow!("{vote_account_pubkey} account does not exist"))?;

    if vote_account.owner != solana_vote_program::id() {
        bail!("{vote_account_pubkey} is not a vote account");
    }

    let vote_state = VoteStateV4::deserialize(&vote_account.data, vote_account_pubkey)
        .map_err(|_| anyhow!("Account data could not be deserialized to vote state"))?;

    let balance_sol = lamports_to_sol(vote_account.lamports);

    let root_slot = match vote_state.root_slot {
        Some(slot) => slot.to_string(),
        None => "~".to_string(),
    };

    let timestamp = chrono::DateTime::from_timestamp(vote_state.last_timestamp.timestamp, 0)
        .expect("Solana timestamp should always be in valid range")
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let vote_authority = vote_state
        .authorized_voters
        .last()
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| vote_state.node_pubkey.to_string());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Field").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Account Balance"),
            Cell::new(format!("{balance_sol} SOL")),
        ])
        .add_row(vec![
            Cell::new("Validator Identity"),
            Cell::new(vote_state.node_pubkey.to_string()),
        ])
        .add_row(vec![Cell::new("Vote Authority"), Cell::new(vote_authority)])
        .add_row(vec![
            Cell::new("Withdraw Authority"),
            Cell::new(vote_state.authorized_withdrawer.to_string()),
        ])
        .add_row(vec![
            Cell::new("Credits"),
            Cell::new(vote_state.credits().to_string()),
        ])
        .add_row(vec![
            Cell::new("Commission"),
            Cell::new(format!(
                "{}%",
                vote_state.inflation_rewards_commission_bps / 100
            )),
        ])
        .add_row(vec![Cell::new("Root Slot"), Cell::new(root_slot)])
        .add_row(vec![
            Cell::new("Recent Timestamp"),
            Cell::new(format!(
                "{} from slot {}",
                timestamp, vote_state.last_timestamp.slot
            )),
        ]);

    println!("\n{}", style("VOTE ACCOUNT INFORMATION").green().bold());
    println!("{table}");

    Ok(())
}

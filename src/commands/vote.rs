use {
    crate::{
        commands::CommandExec, context::ScillaContext, error::ScillaResult, prompt::prompt_data,
        ui::show_spinner,
    },
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_pubkey::Pubkey,
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
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            VoteCommand::ShowVoteAccount => {
                let pubkey: Pubkey = prompt_data("Enter Vote Account Pubkey:")?;
                show_spinner("Show Vote Account", show_vote_account(ctx, &pubkey)).await?;
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

            println!("\n{}", style("VOTE ACCOUNT INFORMATION").green().bold());
            println!("{}", table);
        }
        None => {
            println!(
                "{} Vote account {} not found in current or delinquent validators.",
                style("⚠").yellow(),
                style(pubkey).cyan()
            );
        }
    }

    Ok(())
}

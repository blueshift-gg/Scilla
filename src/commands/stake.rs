use {
    crate::{
        commands::CommandExec,
        context::ScillaContext,
        error::ScillaResult,
        prompt::prompt_data,
        ui::{print_error, show_spinner},
        constants::LAMPORTS_PER_SOL,
    },
    console::style,
    solana_keypair::{Keypair, Signer},
    solana_pubkey::Pubkey,
    solana_stake_interface::{
        instruction::{split, deactivate_stake, withdraw, initialize, delegate_stake, merge},
        state::{Authorized, Lockup, StakeStateV2},
        program::id as stake_program_id,
    },
    solana_transaction::Transaction,
    solana_message::Message,
    solana_system_interface::instruction::create_account,
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
            StakeCommand::Create => {
                let amount: f64 = prompt_data("Enter Amount to Stake (SOL):")?;
                let validator: Pubkey = prompt_data("Enter Validator Vote Account to Delegate to:")?;
                
                let res = show_spinner(self.description(), create_and_delegate_stake(ctx, amount, validator)).await;
                 if let Err(e) = res {
                    print_error(format!("Create & Delegate failed: {}", e));
                }
            }
            StakeCommand::Delegate => {
                let stake_pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey to Delegate:")?;
                let validator: Pubkey = prompt_data("Enter Validator Vote Account Pubkey:")?;
                
                let res = show_spinner(self.description(), delegate_stake_account(ctx, stake_pubkey, validator)).await;
                 if let Err(e) = res {
                    print_error(format!("Delegation failed: {}", e));
                }
            }
            StakeCommand::Deactivate => {
                let stake_pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey to Deactivate:")?;
                let res = show_spinner(self.description(), deactivate_stake_account(ctx, stake_pubkey)).await;
                if let Err(e) = res {
                    print_error(format!("Deactivation failed: {}", e));
                }
            }
            StakeCommand::Withdraw => {
                let stake_pubkey: Pubkey = prompt_data("Enter Stake Account Pubkey to Withdraw from:")?;
                let recipient: Pubkey = prompt_data("Enter Recipient Address:")?;
                let amount: f64 = prompt_data("Enter Amount to Withdraw (SOL):")?;
                
                let res = show_spinner(self.description(), withdraw_stake(ctx, stake_pubkey, recipient, amount)).await;
                if let Err(e) = res {
                    print_error(format!("Withdrawal failed: {}", e));
                }
            }
            StakeCommand::Merge => {
                 let dest_pubkey: Pubkey = prompt_data("Enter Destination Stake Account Pubkey (to keep):")?;
                 let src_pubkey: Pubkey = prompt_data("Enter Source Stake Account Pubkey (to drain):")?;
                 
                 let res = show_spinner(self.description(), merge_stake(ctx, dest_pubkey, src_pubkey)).await;
                 if let Err(e) = res {
                    print_error(format!("Merge failed: {}", e));
                }
            }
            StakeCommand::Split => {
                let stake_pubkey: Pubkey = prompt_data("Enter Source Stake Account Pubkey:")?;
                let amount: f64 = prompt_data("Enter Amount to Split (SOL):")?;
                
                let res = show_spinner(self.description(), split_stake(ctx, stake_pubkey, amount)).await;
                if let Err(e) = res {
                    print_error(format!("Split failed: {}", e));
                }
            }
            StakeCommand::Show => todo!(),
            StakeCommand::History => todo!(),
            StakeCommand::GoBack => return Ok(CommandExec::GoBack),
        }
        
        Ok(CommandExec::Process(()))
    }
}

async fn split_stake(ctx: &ScillaContext, source_stake_pubkey: Pubkey, amount_sol: f64) -> anyhow::Result<()> {
    let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
    
    // Generate a new keypair for the split stake account
    let split_stake_keypair = Keypair::new();
    let split_stake_pubkey = split_stake_keypair.pubkey();

    let authorized_pubkey = ctx.pubkey(); // Assuming wallet is the staker authority
    
    // Create split instruction
    let instruction = split(
        &source_stake_pubkey,
        &authorized_pubkey,
        lamports,
        &split_stake_pubkey,
    );
    
    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    
    let message = Message::new(&instruction, Some(&authorized_pubkey));
    
    // Sign with wallet (authority) AND the new stake account keypair
    let transaction = Transaction::new(
        &[ctx.keypair(), &split_stake_keypair],
        message,
        recent_blockhash,
    );
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
    println!(
        "\n{} {}\n{}\n{}",
        style("Stake Split Successful!").green().bold(),
        style(format!("Signature: {}", signature)).cyan(),
        style(format!("New Stake Account: {}", split_stake_pubkey)).yellow().bold(),
        style(format!("Split Amount: {} SOL", amount_sol)).cyan()
    );

    Ok(())
}

async fn deactivate_stake_account(ctx: &ScillaContext, stake_pubkey: Pubkey) -> anyhow::Result<()> {
    let authorized_pubkey = ctx.pubkey();
    let instruction = deactivate_stake(&stake_pubkey, &authorized_pubkey);
    
    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    let message = Message::new(&[instruction], Some(authorized_pubkey));
    let transaction = Transaction::new(&[ctx.keypair()], message, recent_blockhash);
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
    println!(
        "\n{} {}\n{}",
        style("Stake Deactivated Successfully!").green().bold(),
        style(format!("Stake Account: {}", stake_pubkey)).yellow(),
        style(format!("Signature: {}", signature)).cyan()
    );
    
    Ok(())
}

async fn withdraw_stake(ctx: &ScillaContext, stake_pubkey: Pubkey, recipient: Pubkey, amount_sol: f64) -> anyhow::Result<()> {
    let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
    let authorized_pubkey = ctx.pubkey();
    
    let instruction = withdraw(
        &stake_pubkey,
        &authorized_pubkey,
        &recipient,
        lamports,
        None, // Custodian
    );
    
    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    let message = Message::new(&[instruction], Some(authorized_pubkey));
    let transaction = Transaction::new(&[ctx.keypair()], message, recent_blockhash);
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
     println!(
        "\n{} {}\n{}\n{}",
        style("Stake Withdrawn Successfully!").green().bold(),
        style(format!("From Stake Account: {}", stake_pubkey)).yellow(),
        style(format!("To Recipient: {}", recipient)).yellow(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

async fn create_and_delegate_stake(ctx: &ScillaContext, amount_sol: f64, validator_vote_pubkey: Pubkey) -> anyhow::Result<()> {
    let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
    let from_pubkey = ctx.pubkey();
    let stake_keypair = Keypair::new();
    let stake_pubkey = stake_keypair.pubkey();

    let space = std::mem::size_of::<StakeStateV2>(); 
    let rent = ctx.rpc().get_minimum_balance_for_rent_exemption(space).await?;
    let total_lamports = lamports + rent;

    let create_account_ix = create_account(
        &from_pubkey,
        &stake_pubkey,
        total_lamports,
        space as u64,
        &stake_program_id(),
    );

    let authorized = Authorized {
        staker: *from_pubkey,
        withdrawer: *from_pubkey,
    };
    let lockup = Lockup::default();
    let initialize_ix = initialize(
        &stake_pubkey,
        &authorized,
        &lockup,
    );

    let delegate_ix = delegate_stake(
        &stake_pubkey,
        &from_pubkey,
        &validator_vote_pubkey,
    );

    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    
    let message = Message::new(
        &[create_account_ix, initialize_ix, delegate_ix], 
        Some(&from_pubkey)
    );
    
    let transaction = Transaction::new(
        &[ctx.keypair(), &stake_keypair],
        message,
        recent_blockhash,
    );
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
    println!(
        "\n{} {}\n{}\n{}",
        style("Stake Created & Delegated!").green().bold(),
        style(format!("New Stake Account: {}", stake_pubkey)).yellow().bold(),
         style(format!("Validator: {}", validator_vote_pubkey)).cyan(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

async fn delegate_stake_account(ctx: &ScillaContext, stake_pubkey: Pubkey, validator_vote_pubkey: Pubkey) -> anyhow::Result<()> {
    let authorized_pubkey = ctx.pubkey();
    let instruction = delegate_stake(
        &stake_pubkey,
        &authorized_pubkey,
        &validator_vote_pubkey,
    );
    
    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;
    let message = Message::new(&[instruction], Some(&authorized_pubkey));
    let transaction = Transaction::new(&[ctx.keypair()], message, recent_blockhash);
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
     println!(
        "\n{} {}\n{}",
        style("Stake Delegated Successfully!").green().bold(),
        style(format!("Validator: {}", validator_vote_pubkey)).cyan(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

async fn merge_stake(ctx: &ScillaContext, dest_pubkey: Pubkey, src_pubkey: Pubkey) -> anyhow::Result<()> {
    let authorized_pubkey = ctx.pubkey();
    
    let instructions = merge(
        &dest_pubkey,
        &src_pubkey,
        &authorized_pubkey,
    );

    let recent_blockhash = ctx.rpc().get_latest_blockhash().await?;

    let message = Message::new(&instructions, Some(&authorized_pubkey));
    let transaction = Transaction::new(&[ctx.keypair()], message, recent_blockhash);
    
    let signature = ctx.rpc().send_and_confirm_transaction(&transaction).await?;
    
     println!(
        "\n{} {}\n{}",
        style("Stake Merged Successfully!").green().bold(),
        style(format!("Merged {} into {}", src_pubkey, dest_pubkey)).yellow(),
        style(format!("Signature: {}", signature)).cyan()
    );

    Ok(())
}

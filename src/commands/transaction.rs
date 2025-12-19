use {
    crate::{
        commands::CommandExec,
        context::ScillaContext,
        error::ScillaResult,
        prompt::prompt_data,
        ui::show_spinner,
    },
    solana_signature::Signature,
    std::fmt,
};

/// Commands related to transaction operations
#[derive(Debug, Clone)]
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
            Self::GoBack => "Go Back",
        })
    }
}

impl TransactionCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            Self::CheckConfirmation => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(self.spinner_msg(), process_check_confirmation(ctx, &signature)).await?;
            }
            Self::FetchStatus => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(self.spinner_msg(), process_fetch_status(ctx, &signature)).await?;
            }
            Self::FetchTransaction => {
                let signature: Signature = prompt_data("Enter transaction signature:")?;
                show_spinner(self.spinner_msg(), process_fetch_transaction(ctx, &signature)).await?;
            }
            Self::SendTransaction => {
                show_spinner(self.spinner_msg(), process_send_transaction(ctx)).await?;
            }
            Self::GoBack => return Ok(CommandExec::GoBack),
        }

        Ok(CommandExec::Process(()))
    }
}
async fn process_check_confirmation(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()>  {
    // Implementation for checking transaction confirmation
    Ok(())
}
async fn process_fetch_status(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()>  {
    // Implementation for fetching transaction status
    Ok(())
}
async fn process_fetch_transaction(ctx: &ScillaContext, signature: &Signature) -> anyhow::Result<()> {
    // Implementation for fetching full transaction data
    Ok(())
}
async fn process_send_transaction(ctx: &ScillaContext) -> anyhow::Result<()> {
    // Implementation for sending a transaction
    Ok(())
}
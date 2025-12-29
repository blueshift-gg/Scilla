use {
    crate::{
        commands::CommandExec,
        context::ScillaContext,
        error::ScillaResult,
        misc::helpers::lamports_to_sol,
        prompt::prompt_data,
        ui::show_spinner,
    },
    anyhow::bail,
    comfy_table::{Cell, Table, presets::UTF8_FULL},
    console::style,
    solana_pubkey::Pubkey,
    solana_rpc_client_api::config::RpcTokenAccountsFilter,
    std::fmt,
};

/// SPL Token commands listed here
#[derive(Debug, Clone)]
pub enum TokenCommand {
    ListTokenAccounts,
    TokenBalance,
    MintInfo,
    GoBack,
}


impl TokenCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            TokenCommand::ListTokenAccounts => "Fetching token accounts…",
            TokenCommand::TokenBalance => "Fetching token balance…",
            TokenCommand::MintInfo => "Fetching mint info…",
            TokenCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for TokenCommand { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cmd = match self {
            TokenCommand::ListTokenAccounts => "List token accounts",
            TokenCommand::TokenBalance => "Token balance",
            TokenCommand::MintInfo => "Mint info",
            TokenCommand::GoBack => "Go back",
        };
        write!(f, "{cmd}")
    }
}

impl TokenCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> ScillaResult<()> {
        match self {
            TokenCommand::ListTokenAccounts => {
                show_spinner(self.spinner_msg(), list_token_accounts(ctx)).await?;
            }
            TokenCommand::TokenBalance => {
                let token_account: Pubkey = prompt_data("Enter token account pubkey:")?;
                show_spinner(self.spinner_msg(), fetch_token_balance(ctx, &token_account)).await?;
            }
            TokenCommand::MintInfo => {
                let mint: Pubkey = prompt_data("Enter mint address:")?;
                show_spinner(self.spinner_msg(), fetch_mint_info(ctx, &mint)).await?;
            }
            TokenCommand::GoBack => return Ok(CommandExec::GoBack),
        }
        Ok(CommandExec::Process(()))
    }
}
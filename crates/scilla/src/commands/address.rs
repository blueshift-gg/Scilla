use {
    crate::{
        commands::{
            Command, CommandFlow,
            navigation::{NavigationSection, NavigationTarget},
        },
        context::ScillaContext,
        misc::helpers::read_keypair_from_path,
        prompt::{prompt_input_data, prompt_keypair_path},
    },
    console::style,
    solana_keypair::Signer,
    solana_pubkey::Pubkey,
    std::fmt,
};

/// Commands related to keypair and address utilities
#[derive(Debug, Clone, Copy)]
pub enum AddressCommand {
    Address,
    DerivePda,
    GoBack,
}

impl fmt::Display for AddressCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            AddressCommand::Address => "Get address",
            AddressCommand::DerivePda => "Derive PDA",
            AddressCommand::GoBack => "Go back",
        };
        write!(f, "{command}")
    }
}

impl Command for AddressCommand {
    async fn process_command(&self, ctx: &mut ScillaContext) -> anyhow::Result<CommandFlow> {
        ctx.get_nav_context_mut()
            .checked_push(NavigationSection::Address);
        match self {
            AddressCommand::Address => {
                let path = prompt_keypair_path("Enter keypair path:", ctx);
                let keypair = read_keypair_from_path(&path)?;
                println!(
                    "{} {}\n{} {}",
                    style("Keypair path:").green().bold(),
                    style(path.display()).cyan(),
                    style("Address:").green().bold(),
                    style(keypair.pubkey()).cyan()
                );
            }
            AddressCommand::DerivePda => {
                let program_id: Pubkey = prompt_input_data("Enter program ID:");
                let seeds_input: String = prompt_input_data("Enter comma-separated string seeds:");
                let seed_strings: Vec<String> = seeds_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let seed_refs: Vec<&[u8]> = seed_strings.iter().map(|s| s.as_bytes()).collect();
                let (pda, bump) = Pubkey::find_program_address(&seed_refs, &program_id);
                println!(
                    "{} {}\n{} {}",
                    style("PDA:").green().bold(),
                    style(pda).cyan(),
                    style("Bump:").green().bold(),
                    style(bump).cyan()
                );
            }
            AddressCommand::GoBack => {
                return Ok(CommandFlow::NavigateTo(NavigationTarget::PreviousSection));
            }
        }

        Ok(CommandFlow::Processed)
    }
}

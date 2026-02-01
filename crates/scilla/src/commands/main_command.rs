use {
    crate::{
        commands::{Command, CommandFlow, navigation::NavigationSection},
        context::ScillaContext,
        prompt::{
            prompt_account_section, prompt_build_section, prompt_cluster_section,
            prompt_config_section, prompt_program_section, prompt_stake_section,
            prompt_transaction_section, prompt_vote_section,
        },
    },
    anyhow::Ok,
    std::fmt,
};

pub enum MainCommand {
    Account,
    Cluster,
    Stake,
    Program,
    Vote,
    Transaction,
    Build,
    ScillaConfig,
    Exit,
}

impl fmt::Display for MainCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MainCommand::Account => "Account",
            MainCommand::Cluster => "Cluster",
            MainCommand::Stake => "Stake",
            MainCommand::Program => "Program",
            MainCommand::Vote => "Vote",
            MainCommand::Transaction => "Transaction",
            MainCommand::ScillaConfig => "Scilla Config",
            MainCommand::Build => "Build",
            MainCommand::Exit => "Exit",
        };
        f.write_str(label)
    }
}

impl Command for MainCommand {
    async fn process_command(&self, ctx: &mut ScillaContext) -> anyhow::Result<CommandFlow> {
        ctx.get_nav_context_mut()
            .checked_push(NavigationSection::Main);

        let flow = match self {
            MainCommand::Cluster => prompt_cluster_section()?.process_command(ctx).await?,
            MainCommand::Stake => prompt_stake_section()?.process_command(ctx).await?,
            MainCommand::Account => prompt_account_section()?.process_command(ctx).await?,
            MainCommand::Vote => prompt_vote_section()?.process_command(ctx).await?,
            MainCommand::Transaction => prompt_transaction_section()?.process_command(ctx).await?,
            MainCommand::Program => prompt_program_section()?.process_command(ctx).await?,
            MainCommand::Build => prompt_build_section()?.process_command(ctx).await?,
            MainCommand::ScillaConfig => prompt_config_section()?.process_command(ctx).await?,
            MainCommand::Exit => {
                return Ok(CommandFlow::Exit);
            }
        };

        Ok(flow)
    }
}

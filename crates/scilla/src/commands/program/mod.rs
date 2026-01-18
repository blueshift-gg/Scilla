use {
    crate::{
        commands::{
            Command, CommandFlow,
            navigation::{NavigationSection, NavigationTarget},
        },
        context::ScillaContext,
        prompt::{prompt_go_back, prompt_program_section_shared},
    },
    anyhow::Ok,
    core::fmt,
};

mod build;
mod close;
mod deploy;
mod extend;
mod upgrade;

#[derive(Debug, Clone, Copy)]
pub enum ProgramCommand {
    ProgramLegacy,
    ProgramV4,
    GoBack,
}

#[derive(Debug, Clone, Copy)]
pub enum ProgramShared {
    Deploy,
    Upgrade,
    Build,
    Close,
    Extend,
    GoBack,
}

impl ProgramShared {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ProgramShared::Deploy => "Deploying program",
            ProgramShared::Upgrade => "Upgrading program",
            ProgramShared::Build => "Building program",
            ProgramShared::Close => "Closing program",
            ProgramShared::Extend => "Extending program data",
            ProgramShared::GoBack => "Go Back",
        }
    }
}

impl fmt::Display for ProgramShared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ProgramShared::Deploy => "Deploy",
            ProgramShared::Upgrade => "Upgrade",
            ProgramShared::Build => "Build",
            ProgramShared::Close => "Close",
            ProgramShared::Extend => "Extend",
            ProgramShared::GoBack => "Go Back",
        };

        f.write_str(label)
    }
}

impl ProgramShared {
    fn process_command(&self, _ctx: &ScillaContext) -> CommandFlow {
        match self {
            ProgramShared::Deploy => todo!(),
            ProgramShared::Upgrade => todo!(),
            ProgramShared::Build => todo!(),
            ProgramShared::Close => todo!(),
            ProgramShared::Extend => todo!(),
            ProgramShared::GoBack => CommandFlow::NavigateTo(prompt_go_back()),
        }
    }
}

impl ProgramCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ProgramCommand::ProgramLegacy => "Managing legacy program",
            ProgramCommand::ProgramV4 => "Managing v4 program",
            ProgramCommand::GoBack => "Going back...",
        }
    }
}

impl fmt::Display for ProgramCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ProgramCommand::ProgramLegacy => "Legacy Program",
            ProgramCommand::ProgramV4 => "V4 Program",
            ProgramCommand::GoBack => "Go Back",
        };

        f.write_str(label)
    }
}

impl Command for ProgramCommand {
    async fn process_command(&self, ctx: &mut ScillaContext) -> anyhow::Result<CommandFlow> {
        ctx.get_nav_context_mut()
            .checked_push(NavigationSection::Program);
        let res = match self {
            ProgramCommand::ProgramLegacy => {
                ctx.get_nav_context_mut()
                    .checked_push(NavigationSection::ProgramLegacy);
                let command = prompt_program_section_shared()?;
                command.process_command(ctx)
            }
            ProgramCommand::ProgramV4 => {
                ctx.get_nav_context_mut()
                    .checked_push(NavigationSection::ProgramV4);
                let command = prompt_program_section_shared()?;
                command.process_command(ctx)
            }
            ProgramCommand::GoBack => {
                return Ok(CommandFlow::NavigateTo(NavigationTarget::PreviousSection));
            }
        };
        Ok(res)
    }
}

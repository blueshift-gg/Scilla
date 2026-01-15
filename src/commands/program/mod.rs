use {
    crate::{commands::CommandFlow, context::ScillaContext},
    core::fmt,
};

mod build;
mod close;
mod deploy;
mod extend;
mod upgrade;

pub use deploy::deploy;

#[derive(Debug, Clone)]
pub enum ProgramCommand {
    Deploy,
    Upgrade,
    Build,
    Close,
    Extend,
    GoBack,
}

impl ProgramCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ProgramCommand::Deploy => "Deploying program",
            ProgramCommand::Upgrade => "Upgrading program",
            ProgramCommand::Build => "Building program",
            ProgramCommand::Close => "Closing program",
            ProgramCommand::Extend => "Extending program data",
            ProgramCommand::GoBack => "Going back...",
        }
    }
}

impl fmt::Display for ProgramCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match self {
            ProgramCommand::Deploy => "Deploy",
            ProgramCommand::Upgrade => "Upgrade",
            ProgramCommand::Build => "Build",
            ProgramCommand::Close => "Close",
            ProgramCommand::Extend => "Extend",
            ProgramCommand::GoBack => "Go Back",
        };
        write!(f, "{command}")
    }
}

impl ProgramCommand {
    pub async fn process_command(&self, ctx: &ScillaContext) -> CommandFlow<()> {
        match self {
            // import here the functions we build in the files
            ProgramCommand::Deploy => deploy(ctx).await,
            ProgramCommand::Upgrade => todo!(),
            ProgramCommand::Build => todo!(),
            ProgramCommand::Close => todo!(),
            ProgramCommand::Extend => todo!(),
            ProgramCommand::GoBack => CommandFlow::GoBack,
        }
    }
}

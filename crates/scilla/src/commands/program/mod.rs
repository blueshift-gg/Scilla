use {
    crate::{
        commands::{CommandFlow, NavigationTarget},
        context::ScillaContext,
    },
    core::fmt,
};

mod build;
mod close;
mod deploy;
mod extend;
mod upgrade;

// pub use build::*;
// pub use close::*;
// pub use deploy::*;
// pub use extend::*;
// pub use upgrade::*;

#[derive(Debug)]
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
    pub async fn process_command(&self, _ctx: &ScillaContext) -> CommandFlow {
        match self {
            // import here the functions we build in the files
            ProgramCommand::Deploy => todo!(),
            ProgramCommand::Upgrade => todo!(),
            ProgramCommand::Build => todo!(),
            ProgramCommand::Close => todo!(),
            ProgramCommand::Extend => todo!(),
            ProgramCommand::GoBack => CommandFlow::NavigateTo(NavigationTarget::MainSection),
        }
    }
}

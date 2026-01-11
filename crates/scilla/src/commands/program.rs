use {
    crate::{ScillaContext, commands::CommandFlow},
    std::fmt,
};

/// Commands related to BPF Loader Upgradeable program operations
#[derive(Debug, Clone)]
pub enum ProgramCommand {
    Extend,
    GoBack,
}

impl ProgramCommand {
    pub fn spinner_msg(&self) -> &'static str {
        match self {
            ProgramCommand::Extend => "Extending program account…",
            ProgramCommand::GoBack => "Going back…",
        }
    }
}

impl fmt::Display for ProgramCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ProgramCommand::Extend => "Extend program",
            ProgramCommand::GoBack => "Go back",
        };
        write!(f, "{text}")
    }
}

impl ProgramCommand {
    pub async fn process_command(&self, _ctx: &ScillaContext) -> CommandFlow<()> {
        match self {
            ProgramCommand::Extend => {
                // TODO: Implement program extend functionality
                println!("Program extend functionality - Coming soon!");
            }
            ProgramCommand::GoBack => return CommandFlow::GoBack,
        }

        CommandFlow::Process(())
    }
}

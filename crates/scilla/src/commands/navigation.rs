use {
    crate::{
        commands::{Command, CommandFlow, program::ProgramCommand},
        context::ScillaContext,
        prompt::{
            prompt_account_section, prompt_cluster_section, prompt_config_section,
            prompt_main_section, prompt_program_section, prompt_stake_section,
            prompt_transaction_section, prompt_vote_section,
        },
    },
    std::fmt::{self, Display},
};

pub enum NavigationTarget {
    MainSection,
    PreviousSection,
}

/// Maximum nesting depth for navigation (root + subsections).
/// Adjust this constant if deeper menu nesting is needed.
const MAX_INTERACTION_DEPTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationSection {
    Main,
    Account,
    Cluster,

    // Program with subcommands
    Program,
    ProgramLegacy,
    ProgramV4,

    Stake,
    Vote,
    Transaction,
    ScillaConfig,
    Exit,
}

impl Display for NavigationSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            NavigationSection::Main => "Main",
            NavigationSection::Account => "Account",
            NavigationSection::Cluster => "Cluster",
            NavigationSection::Program => "Program",
            NavigationSection::ProgramLegacy => "ProgramLegacy",
            NavigationSection::ProgramV4 => "ProgramV4",
            NavigationSection::Stake => "Stake",
            NavigationSection::Vote => "Vote",
            NavigationSection::Transaction => "Transaction",
            NavigationSection::ScillaConfig => "Scilla Config",
            NavigationSection::Exit => "Exit",
        };

        f.write_str(label)
    }
}

impl NavigationSection {
    pub async fn prompt_and_process_command(
        &self,
        ctx: &mut ScillaContext,
    ) -> anyhow::Result<crate::commands::CommandFlow> {
        match self {
            NavigationSection::Main => {
                let cmd = prompt_main_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::Account => {
                let cmd = prompt_account_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::Cluster => {
                let cmd = prompt_cluster_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::Program => {
                let cmd = prompt_program_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::ProgramLegacy => {
                ProgramCommand::ProgramLegacy.process_command(ctx).await
            }

            NavigationSection::ProgramV4 => ProgramCommand::ProgramV4.process_command(ctx).await,

            NavigationSection::Stake => {
                let cmd = prompt_stake_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::Vote => {
                let cmd = prompt_vote_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::Transaction => {
                let cmd = prompt_transaction_section()?;
                cmd.process_command(ctx).await
            }

            NavigationSection::ScillaConfig => {
                let cmd = prompt_config_section()?;
                cmd.process_command(ctx).await // this is sync 
            }

            NavigationSection::Exit => Ok(CommandFlow::Exit),
        }
    }
}

/// Tracks the current navigation state as a stack of sections.
#[derive(Debug, Clone)]
pub struct NavContext {
    stack: Vec<NavigationSection>,
}

impl Default for NavContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NavContext {
    /// Creates an empty navigation context.
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(MAX_INTERACTION_DEPTH);
        stack.push(NavigationSection::Main);
        Self { stack }
    }

    /// Navigates back to the parent section.
    ///
    /// Returns `None` if already at MainSection (stack has only one element),
    /// preserving the root section.
    pub fn pop_and_get_previous(&mut self) -> Option<NavigationSection> {
        self.stack.pop();
        self.current()
    }

    /// Returns the current active section, or `None` if uninitialized.
    pub fn current(&self) -> Option<NavigationSection> {
        self.stack.last().copied()
    }

    /// Resets the navigation stack and sets the provided section as the current
    /// root.
    pub fn reset_navigation_to(&mut self, section: NavigationSection) {
        self.stack.clear();
        self.stack.push(section);
    }

    /// Resets the navigation stack and sets the provided section as the current
    /// root.
    pub fn reset_navigation_to_main(&mut self) {
        self.reset_navigation_to(NavigationSection::Main);
    }

    /// Pushes a new section onto the navigation stack.
    ///
    /// # Panics
    /// Panics if the maximum navigation depth is exceeded.
    pub fn checked_push(&mut self, section: NavigationSection) {
        if self.stack.contains(&section) {
            return;
        }
        if self.stack.len() == MAX_INTERACTION_DEPTH {
            panic!("navigation stack exceeded");
        }

        self.stack.push(section);
    }

    /// Returns `true` if the current section is nested within another section.
    pub fn is_nested(&self) -> bool {
        self.stack.len() >= 3
    }
}

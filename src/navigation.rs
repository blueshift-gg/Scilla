use crate::commands::CommandGroup;

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationContext {
    MainMenu,
    AccountMenu,
    ClusterMenu,
    StakeMenu,
    VoteMenu,
    TransactionMenu,
    ConfigMenu,
}

impl NavigationContext {
    pub fn from_command_group(group: &CommandGroup) -> Self {
        match group {
            CommandGroup::Account => NavigationContext::AccountMenu,
            CommandGroup::Cluster => NavigationContext::ClusterMenu,
            CommandGroup::Stake => NavigationContext::StakeMenu,
            CommandGroup::Vote => NavigationContext::VoteMenu,
            CommandGroup::Transaction => NavigationContext::TransactionMenu,
            CommandGroup::ScillaConfig => NavigationContext::ConfigMenu,
            CommandGroup::Exit => NavigationContext::MainMenu,
        }
    }
}

pub struct NavigationStack {
    stack: Vec<NavigationContext>,
}

impl NavigationStack {
    pub fn new() -> Self {
        Self {
            stack: vec![NavigationContext::MainMenu],
        }
    }

    pub fn push(&mut self, context: NavigationContext) {
        self.stack.push(context);
    }

    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn current(&self) -> &NavigationContext {
        self.stack
            .last()
            .expect("Navigation stack should never be empty")
    }

    pub fn is_at_main_menu(&self) -> bool {
        self.stack.len() == 1 && matches!(self.current(), NavigationContext::MainMenu)
    }
}

impl Default for NavigationStack {
    fn default() -> Self {
        Self::new()
    }
}

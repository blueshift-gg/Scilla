/// A section is the top-level menu entry (Account, Config, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSection {
    Account,
    Cluster,
    Config,
    Stake,
    Transaction,
    Vote,
}

impl CommandSection {
    /// Returns the maximum navigation depth for this section.
    ///
    /// Depth model:
    /// - 0: Main menu (handled by `AppNav::MainMenu`)
    /// - 1: Section root (command selection)
    /// - 2+: Nested user prompts within a command
    ///
    /// Formula: `max_depth = 1 + max_prompts_in_section`
    pub const fn max_depth(self) -> usize {
        match self {
            CommandSection::Cluster => 1,     // 0 Prompts
            CommandSection::Account => 2,     // 1 Prompt
            CommandSection::Config => 3,      // 2 Prompts
            CommandSection::Transaction => 3, // 2 Prompts
            CommandSection::Vote => 5,        // 4 Prompts
            CommandSection::Stake => 8,       // 7 Prompts
        }
    }
}

/// Section-scoped bounded stack, implemented as a depth index.
/// Main menu is represented by `AppNav::MainMenu`.
/// InSection depths are 1..=max_depth:
/// - depth 1: section root (command selection)
/// - depth 2+: nested user prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSectionNav {
    cmd_section: CommandSection,
    depth: usize,
}

impl CommandSectionNav {
    /// Create a new navigation state for a section.
    pub const fn new(section: CommandSection) -> Self {
        Self {
            cmd_section: section,
            depth: 1,
        }
    }

    /// Reset navigation to section root (depth 1).
    pub fn reset(&mut self) {
        self.depth = 1;
    }

    /// Forward navigation inside the section.
    /// Returns false if at max depth.
    #[must_use]
    pub fn push(&mut self) -> bool {
        if self.at_max_depth() {
            return false;
        }
        self.depth += 1;
        true
    }

    /// Backward navigation inside the section.
    /// Returns false if at root.
    #[must_use]
    pub fn pop(&mut self) -> bool {
        if self.at_section_root() {
            return false;
        }
        self.depth -= 1;
        true
    }

    /// Returns true if at section root (depth 1).
    pub const fn at_section_root(&self) -> bool {
        self.depth == 1
    }

    /// Returns true if at max depth for this section.
    pub const fn at_max_depth(&self) -> bool {
        self.depth >= self.cmd_section.max_depth()
    }

    pub const fn section(&self) -> CommandSection {
        self.cmd_section
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }
}
/// Define the state we're on the navigation context.
/// Main menu or within a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppNav {
    MainMenu,
    InSection(CommandSectionNav),
}

impl AppNav {
    /// Enter or switch to a section.
    pub fn enter_section(&mut self, section: CommandSection) {
        *self = AppNav::InSection(CommandSectionNav::new(section));
    }

    /// Drop section state and go back to main menu.
    pub fn go_to_menu(&mut self) {
        *self = AppNav::MainMenu;
    }

    /// Unified "Back" behavior:
    /// - pop within section
    /// - go to main menu if at root
    /// - no-op if already at main menu (intentional)
    pub fn go_back(&mut self) {
        match self {
            AppNav::MainMenu => (),
            AppNav::InSection(state) => {
                if !state.pop() {
                    *self = AppNav::MainMenu;
                }
            }
        }
    }

    /// Forward navigation inside a section.
    /// Returns false if at main menu or max depth.
    #[must_use]
    pub fn forward(&mut self) -> bool {
        match self {
            AppNav::MainMenu => false,
            AppNav::InSection(state) => state.push(),
        }
    }

    /// Get the current section.
    pub fn section(&self) -> Option<CommandSection> {
        match self {
            AppNav::MainMenu => None,
            AppNav::InSection(state) => Some(state.section()),
        }
    }

    pub fn section_depth(&self) -> Option<usize> {
        match self {
            AppNav::MainMenu => None,
            AppNav::InSection(state) => Some(state.depth()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> CommandSectionNav {
        let section = CommandSection::Account;
        CommandSectionNav::new(section)
    }

    #[test]
    fn test_max_depth() {
        assert_eq!(CommandSection::Cluster.max_depth(), 1);
        assert_eq!(CommandSection::Account.max_depth(), 2);
        assert_eq!(CommandSection::Config.max_depth(), 3);
        assert_eq!(CommandSection::Transaction.max_depth(), 3);
        assert_eq!(CommandSection::Vote.max_depth(), 5);
        assert_eq!(CommandSection::Stake.max_depth(), 8);
    }

    #[test]
    fn nav_state_new() {
        let nav_state = setup();

        assert_eq!(nav_state.section(), CommandSection::Account);
        assert_eq!(nav_state.depth(), 1);
        assert!(nav_state.at_section_root());
    }

    #[test]
    fn nav_state_forward() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 2);
    }

    #[test]
    fn nav_state_backward() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 2);

        assert!(nav_state.pop());

        assert_eq!(nav_state.depth(), 1);
        assert!(nav_state.at_section_root());
    }

    #[test]
    fn nav_state_reset() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 2);

        nav_state.reset();

        assert_eq!(nav_state.depth(), 1);
        assert!(nav_state.at_section_root());
    }

    #[test]
    fn nav_state_section() {
        let nav_state = setup();

        assert_eq!(nav_state.section(), CommandSection::Account);
    }

    #[test]
    fn app_nav() {
        let mut app_nav = AppNav::MainMenu;

        assert_eq!(app_nav, AppNav::MainMenu);

        app_nav.enter_section(CommandSection::Account);

        assert_eq!(app_nav.section(), Some(CommandSection::Account));

        app_nav.go_to_menu();

        assert_eq!(app_nav, AppNav::MainMenu);
    }

    #[test]
    fn app_nav_enter_section() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);

        assert_eq!(app_nav.section(), Some(CommandSection::Account));
        assert_eq!(app_nav.section_depth(), Some(1));
    }

    #[test]
    fn app_nav_go_menu() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);

        app_nav.go_to_menu();

        assert_eq!(app_nav, AppNav::MainMenu);
        assert_eq!(app_nav.section_depth(), None)
    }

    #[test]
    fn app_nav_go_back() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);

        app_nav.go_back();

        assert_eq!(app_nav, AppNav::MainMenu);
        assert_eq!(app_nav.section_depth(), None)
    }

    #[test]
    fn app_nav_forward() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);

        assert!(app_nav.forward());

        assert_eq!(app_nav.section(), Some(CommandSection::Account));
        assert_eq!(app_nav.section_depth(), Some(2));
    }

    #[test]
    fn nav_state_push_at_max_depth() {
        let mut nav_state = setup();
        assert!(!nav_state.at_max_depth());

        // Account max_depth is 2, starts at 1
        assert!(nav_state.push());
        assert_eq!(nav_state.depth(), 2);
        assert!(nav_state.at_max_depth());

        // Should fail at max
        assert!(!nav_state.push());
        assert_eq!(nav_state.depth(), 2);
    }

    #[test]
    fn nav_state_pop_at_root() {
        let mut nav_state = setup();
        assert!(nav_state.at_section_root());
        assert_eq!(nav_state.depth(), 1);

        assert!(!nav_state.pop());
        assert_eq!(nav_state.depth(), 1);
    }

    #[test]
    fn app_nav_forward_at_main_menu() {
        let mut app_nav = AppNav::MainMenu;
        assert!(!app_nav.forward());
        assert_eq!(app_nav, AppNav::MainMenu);
    }

    #[test]
    fn app_nav_forward_at_max_depth() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);

        // Account: max_depth 2, starts at 1
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(2));

        // Should fail at max
        assert!(!app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(2));
    }

    #[test]
    fn app_nav_go_back_at_main_menu() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.go_back();
        assert_eq!(app_nav, AppNav::MainMenu);
    }

    #[test]
    fn app_nav_go_back_from_nested_depth() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Stake); // max_depth 8

        assert!(app_nav.forward());
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(3));

        app_nav.go_back();
        assert_eq!(app_nav.section_depth(), Some(2));
        assert_eq!(app_nav.section(), Some(CommandSection::Stake));
    }

    #[test]
    fn app_nav_switch_section() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(2));

        // Switch directly to another section
        app_nav.enter_section(CommandSection::Stake);
        assert_eq!(app_nav.section(), Some(CommandSection::Stake));
        assert_eq!(app_nav.section_depth(), Some(1)); // Reset to section root
    }

    #[test]
    fn app_nav_go_back_depth_two_then_exit() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(CommandSection::Account);
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(2));

        // First go_back: depth 2 -> 1, stays in section
        app_nav.go_back();
        assert_eq!(app_nav.section_depth(), Some(1));
        assert_eq!(app_nav.section(), Some(CommandSection::Account));

        // Second go_back: at root (depth 1) -> exits to main menu
        app_nav.go_back();
        assert_eq!(app_nav, AppNav::MainMenu);
    }
}

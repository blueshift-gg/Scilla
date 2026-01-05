/// A section is the top-level menu entry (Account, Config, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Account,
    Config,
    Stake,
    Transaction,
    Vote,
}

impl Section {
    pub const fn max_depth(self) -> usize {
        match self {
            Section::Account => 6,
            _ => todo!(),
        }
    }
}

/// Section-scoped bounded stack, implemented as a depth index.
/// depth == 0 is reserved for the section root.
/// depth in 1..=max_depth are the nested interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionNav {
    section: Section,
    depth: usize,
}

impl SectionNav {
    /// Create a new navigation state for a section.
    pub const fn new(section: Section) -> Self {
        Self { section, depth: 0 }
    }

    /// Reset navigation to root (index 0).
    pub fn reset(&mut self) {
        self.depth = 0;
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
        if self.at_root() {
            return false;
        }
        self.depth -= 1;
        true
    }

    /// Returns true if at section root (depth 0).
    pub const fn at_root(&self) -> bool {
        self.depth == 0
    }

    /// Returns true if at max depth for this section.
    pub const fn at_max_depth(&self) -> bool {
        self.depth >= self.section.max_depth()
    }

    pub const fn section(&self) -> Section {
        self.section
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
    InSection(SectionNav),
}

impl AppNav {
    /// Enter or switch to a section.
    pub fn enter_section(&mut self, section: Section) {
        *self = AppNav::InSection(SectionNav::new(section));
    }

    /// Drop section state and go back to main menu.
    pub fn go_to_menu(&mut self) {
        *self = AppNav::MainMenu
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
    pub fn forward(&mut self) -> bool {
        match self {
            AppNav::MainMenu => false,
            AppNav::InSection(state) => state.push(),
        }
    }

    /// Get the current section.
    pub fn section(&self) -> Option<Section> {
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

    fn setup() -> SectionNav {
        let section = Section::Account;
        SectionNav::new(section)
    }

    #[test]
    fn test_max_depth() {
        let section = Section::Account;
        assert_eq!(section.max_depth(), 6);
    }

    #[test]
    fn nav_state_new() {
        let nav_state = setup();

        assert_eq!(nav_state.section(), Section::Account);
        assert_eq!(nav_state.depth(), 0);
    }

    #[test]
    fn nav_state_forward() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 1);
    }

    #[test]
    fn nav_state_backward() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 1);

        assert!(nav_state.pop());

        assert_eq!(nav_state.depth(), 0);
    }

    #[test]
    fn nav_state_reset() {
        let mut nav_state = setup();

        assert!(nav_state.push());

        assert_eq!(nav_state.depth(), 1);

        nav_state.reset();

        assert_eq!(nav_state.depth(), 0);
    }

    #[test]
    fn nav_state_section() {
        let nav_state = setup();

        assert_eq!(nav_state.section(), Section::Account);
    }

    #[test]
    fn app_nav() {
        let mut app_nav = AppNav::MainMenu;

        assert_eq!(app_nav, AppNav::MainMenu);

        app_nav.enter_section(Section::Account);

        assert_eq!(app_nav.section(), Some(Section::Account));

        app_nav.go_to_menu();

        assert_eq!(app_nav, AppNav::MainMenu);
    }

    #[test]
    fn app_nav_enter_section() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);

        assert_eq!(app_nav.section(), Some(Section::Account));
    }

    #[test]
    fn app_nav_go_menu() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);

        app_nav.go_to_menu();

        assert_eq!(app_nav, AppNav::MainMenu);
        assert_eq!(app_nav.section_depth(), None)
    }

    #[test]
    fn app_nav_go_back() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);

        app_nav.go_back();

        assert_eq!(app_nav, AppNav::MainMenu);
        assert_eq!(app_nav.section_depth(), None)
    }

    #[test]
    fn app_nav_forward() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);

        assert!(app_nav.forward());

        assert_eq!(app_nav.section(), Some(Section::Account));
        assert_eq!(app_nav.section_depth(), Some(1));
    }

    #[test]
    fn nav_state_push_at_max_depth() {
        let mut nav_state = setup();
        assert!(!nav_state.at_max_depth());
        // Push to max depth
        for _ in 0..6 {
            assert!(nav_state.push());
        }
        assert_eq!(nav_state.depth(), 6);
        assert!(nav_state.at_max_depth());
        // Should fail at max
        assert!(!nav_state.push());
        assert_eq!(nav_state.depth(), 6);
    }

    #[test]
    fn nav_state_pop_at_zero() {
        let mut nav_state = setup();
        assert!(nav_state.at_root());
        assert!(!nav_state.pop());
        assert_eq!(nav_state.depth(), 0);
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
        app_nav.enter_section(Section::Account);
        // Push to max depth
        for _ in 0..6 {
            assert!(app_nav.forward());
        }
        assert_eq!(app_nav.section_depth(), Some(6));
        // Should fail at max
        assert!(!app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(6));
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
        app_nav.enter_section(Section::Account);
        assert!(app_nav.forward());
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(2));

        app_nav.go_back();
        assert_eq!(app_nav.section_depth(), Some(1));
        assert_eq!(app_nav.section(), Some(Section::Account));
    }

    #[test]
    fn app_nav_switch_section() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(1));

        // Switch directly to another section
        app_nav.enter_section(Section::Stake);
        assert_eq!(app_nav.section(), Some(Section::Stake));
        assert_eq!(app_nav.section_depth(), Some(0)); // Reset to root
    }

    #[test]
    fn app_nav_go_back_depth_one_then_exit() {
        let mut app_nav = AppNav::MainMenu;
        app_nav.enter_section(Section::Account);
        assert!(app_nav.forward());
        assert_eq!(app_nav.section_depth(), Some(1));

        // First go_back: depth 1 -> 0, stays in section
        app_nav.go_back();
        assert_eq!(app_nav.section_depth(), Some(0));
        assert_eq!(app_nav.section(), Some(Section::Account));

        // Second go_back: at root -> exits to main menu
        app_nav.go_back();
        assert_eq!(app_nav, AppNav::MainMenu);
    }
}

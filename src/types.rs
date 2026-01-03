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
pub struct NavState {
    section: Section,
    depth: usize, 
}

impl NavState {
    pub fn new(section: Section) -> Self {
        Self { section, depth: 0 }
    }

    /// Forward navigation inside the section.
    /// Returns false if at max depth.
    pub fn push(&mut self) -> bool {
        let max = self.section().max_depth();

        if self.depth >= max {
            return false;
        }
        self.depth += 1;
        true
    }

    /// Backward navigation inside the section.
    /// Returns false if at min depth.
    pub fn pop(&mut self) -> bool {
        if self.depth == 0 {
            return false;
        }
        self.depth -= 1;
        true
    }

    /// Reset navigation to root (index 0).
    pub fn reset(&mut self) {
        self.depth = 0;
    }
}

impl NavState {
    pub fn section(&self) -> Section {
        self.section
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

pub enum AppNav {
    MainMenu,
    InSection(NavState),
}

impl AppNav {
    pub fn enter_section(section: Section) -> Self {
        AppNav::InSection(NavState::new(section))
    }
    /// Drop section state and go back to main menu.
    pub fn go_to_menu(&mut self) {
        *self = AppNav::MainMenu
    }

    /// Unified "Back" behavior:
    /// - pop within section
    /// - go to main menu if at root
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
    pub fn forward(&mut self) -> bool {
        match self {
            AppNav::InSection(state) => state.push(),
            AppNav::MainMenu => false,
        }
    }
}

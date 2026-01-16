use crate::commands::CommandGroup;

/// Maximum nesting depth for navigation (root + subsections).
/// Adjust this constant if deeper menu nesting is needed.
const MAX_INTERACTION_DEPTH: usize = 3;

/// Tracks the current navigation state as a stack of sections.
///
/// The stack always maintains at least the root section once set.
/// Navigation flow:
/// - `set_root()` initializes with a top-level section
/// - `current()` returns the active section for prompting
/// - `pop()` navigates back (returns `None` if at root)
/// - `push()` adds a new section to the navigation stack if capacity allows
#[derive(Debug, Clone)]
pub struct NavContext {
    stack: Vec<CommandGroup>,
}

impl Default for NavContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NavContext {
    /// Creates an empty navigation context.
    pub fn new() -> Self {
        let stack = Vec::with_capacity(MAX_INTERACTION_DEPTH);
        Self { stack }
    }

    /// Navigates back to the parent section.
    ///
    /// Returns `None` if already at root (stack has only one element),
    /// preserving the root section.
    pub fn pop(&mut self) -> Option<CommandGroup> {
        if self.stack.len() <= 1 {
            return None;
        }
        self.stack.pop()
    }

    /// Returns the current active section, or `None` if uninitialized.
    pub fn current(&self) -> Option<CommandGroup> {
        self.stack.last().copied()
    }

    /// Sets the root section, clearing any existing navigation history.
    pub fn set_root(&mut self, section: CommandGroup) {
        self.stack.clear();
        self.stack.push(section);
    }

    /// Pushes a new section onto the navigation stack if capacity allows.
    #[must_use]
    pub fn push(&mut self, section: CommandGroup) -> bool {
        if self.stack.len() < MAX_INTERACTION_DEPTH {
            self.stack.push(section);
            return true;
        }
        false
    }

    /// Returns `true` if the current section is nested within another section.
    pub fn is_nested(&self) -> bool {
        self.stack.len() >= 2
    }
}

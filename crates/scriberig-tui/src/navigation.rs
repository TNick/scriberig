//! Maintains presentation navigation history without semantic mutation.

use crate::selection::NavigationLocation;

/// Back/current/forward navigation stacks for the TUI.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NavigationState {
    /// Locations behind the current location.
    pub back: Vec<NavigationLocation>,
    /// The currently displayed location.
    pub current: Option<NavigationLocation>,
    /// Locations ahead of the current location.
    pub forward: Vec<NavigationLocation>,
}

impl NavigationState {
    /// Activates a location, pushing the current one and clearing forward history.
    /// - `location`: Argument named `location`.
    pub fn activate(&mut self, location: NavigationLocation) {
        if let Some(current) = self.current.replace(location) {
            self.back.push(current);
        }
        self.forward.clear();
    }

    /// Moves back one location and reports whether navigation occurred.
    ///
    /// # Returns
    /// The operation result.
    pub fn go_back(&mut self) -> bool {
        let Some(location) = self.back.pop() else {
            return false;
        };
        if let Some(current) = self.current.replace(location) {
            self.forward.push(current);
        }
        true
    }

    /// Moves forward one location and reports whether navigation occurred.
    ///
    /// # Returns
    /// The operation result.
    pub fn go_forward(&mut self) -> bool {
        let Some(location) = self.forward.pop() else {
            return false;
        };
        if let Some(current) = self.current.replace(location) {
            self.back.push(current);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::NavigationState;
    use crate::selection::{NavigationLocation, SemanticReference};

    /// Creates a navigation location for a named semantic reference.
    ///
    /// - `name`: Semantic reference name.
    ///
    /// A navigation location.
    /// # Returns
    /// The operation result.
    fn location(name: &str) -> NavigationLocation {
        NavigationLocation {
            selection: Some(SemanticReference(name.to_owned())),
            ..NavigationLocation::default()
        }
    }

    /// Verifies activation clears forward history.
    #[test]
    fn activation_clears_forward_history() {
        let mut navigation = NavigationState::default();
        navigation.activate(location("one"));
        navigation.activate(location("two"));
        assert!(navigation.go_back());
        navigation.activate(location("three"));
        assert_eq!(navigation.forward.len(), 0);
    }

    /// Verifies back and forward preserve stable locations.
    #[test]
    fn back_and_forward_restore_locations() {
        let mut navigation = NavigationState::default();
        navigation.activate(location("one"));
        navigation.activate(location("two"));
        assert!(navigation.go_back());
        assert_eq!(navigation.current, Some(location("one")));
        assert!(navigation.go_forward());
        assert_eq!(navigation.current, Some(location("two")));
    }
}

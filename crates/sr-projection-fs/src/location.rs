//! Defines normalized filesystem projection locations.

use std::path::{Path, PathBuf};

/// A filesystem location kept separate from semantic identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectionLocation(PathBuf);

impl ProjectionLocation {
    /// Creates a location without filesystem canonicalization.
    ///
    /// # Parameters
    /// - `path`: Relative or absolute path to preserve.
    ///
    /// # Returns
    /// A normalized lexical location.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    if !normalized.pop() {
                        normalized.push(component.as_os_str());
                    }
                }
                _ => normalized.push(component.as_os_str()),
            }
        }
        Self(normalized)
    }

    /// Borrows the represented path.
    ///
    /// # Returns
    /// The lexical path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

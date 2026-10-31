//! Defines normalized filesystem projection locations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A filesystem location kept separate from semantic identity.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ProjectionLocation(PathBuf);

impl ProjectionLocation {
    /// Creates a location without filesystem canonicalization.
    ///
    /// - `path`: Relative or absolute path to preserve.
    ///
    /// A normalized lexical location.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// The lexical path.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

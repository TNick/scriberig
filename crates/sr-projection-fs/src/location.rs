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
        Self(path.into())
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

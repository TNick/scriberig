//! Defines filesystem projection ownership independently of runtime state.

use serde::{Deserialize, Serialize};

/// Declares who may write or import a projected file.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProjectionOwnership {
    ManagedEditable,
    ManagedReadOnly,
    Unmanaged,
    ImportedExternal,
}

impl ProjectionOwnership {
    /// Reports whether Scriberig may automatically write the location.
    #[must_use]
    pub const fn may_write(self) -> bool {
        matches!(self, Self::ManagedEditable)
    }

    /// Reports whether external changes may be imported.
    #[must_use]
    pub const fn may_import(self) -> bool {
        matches!(self, Self::ManagedEditable | Self::ImportedExternal)
    }

    /// Reports whether automatic rewriting is forbidden.
    #[must_use]
    pub const fn forbids_automatic_write(self) -> bool {
        !self.may_write()
    }
}

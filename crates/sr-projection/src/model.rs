//! Defines domain-independent projection identities and classification.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use serde::{Deserialize, Serialize};

use crate::style::StyleRef;

/// Classifies the interaction contract of a projection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectionClass {
    /// Cannot be edited through the projection.
    ReadOnly,
    /// External edits can be parsed and reconciled.
    Editable,
    /// Exists only as a transient representation.
    Ephemeral,
    /// Supports ongoing interactive behavior.
    Interactive,
}

/// Identifies a projection format without closing the set of formats.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ProjectionKind(String);

/// Identifies an external representation without imposing filesystem details.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExternalLocation(String);

/// Identifies one projection instance independently of its location.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ProjectionRef(pub EntityId);

/// Identifies the semantic entity and optional exact revision being projected.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SemanticScopeRef {
    /// Stable entity that owns the projected scope.
    pub entity: EntityId,
    /// Exact semantic revision when the scope is revision-bound.
    pub revision: Option<RevisionId>,
}

/// Identifies a persistent projection specification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ProjectionSpecRef(pub EntityId);

/// Stores user-meaningful projection configuration without a physical location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionSpec {
    /// Semantic scope represented by this specification.
    pub scope: SemanticScopeRef,
    /// Adapter-defined projection format.
    pub kind: ProjectionKind,
    /// Interaction contract for this projection.
    pub class: ProjectionClass,
    /// Exact style input for deterministic rendering.
    pub style: StyleRef,
    /// Stable adapter identifier.
    pub adapter_id: String,
    /// Adapter implementation version.
    pub adapter_version: String,
    /// Renderer implementation version used for deterministic output.
    pub renderer: RendererVersion,
    /// Adapter-defined deterministic configuration values.
    pub configuration: BTreeMap<String, Vec<u8>>,
    /// Generic ownership policy for projected output.
    pub ownership: OwnershipClass,
    /// Optional opaque location for an external representation.
    pub location: Option<ExternalLocation>,
}

/// Classifies ownership without describing a physical location.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OwnershipClass {
    /// Scriberig owns generation and may replace output.
    Managed,
    /// External edits are authoritative inputs for reconciliation.
    UserOwned,
    /// Ownership is shared under an adapter-defined policy.
    Shared,
}

/// Identifies a renderer implementation release.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RendererVersion(pub String);

/// Binds projection rendering or reconciliation to an exact semantic snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SemanticStateRef(pub SnapshotId);

// Provide validated construction and access for extensible projection kinds.
impl ProjectionKind {
    /// Creates a projection kind from its stable adapter-defined identifier.
    ///
    /// - `identifier`: Nonempty identifier for a projection format.
    ///
    /// The kind when its identifier is nonempty.
    ///
    /// Returns an error when the identifier is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(identifier: impl Into<String>) -> Result<Self, &'static str> {
        let identifier = identifier.into();
        if identifier.trim().is_empty() {
            return Err("projection kind identifier must not be empty");
        }
        Ok(Self(identifier))
    }

    /// Returns the stable projection kind identifier.
    ///
    /// - `self`: Kind to inspect.
    ///
    /// The adapter-defined identifier.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ExternalLocation {
    /// Creates an opaque external location reference.
    ///
    /// - `location`: Stable adapter-defined location text.
    ///
    /// The location when it is nonempty.
    ///
    /// Returns an error when the location is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(location: impl Into<String>) -> Result<Self, &'static str> {
        let location = location.into();
        if location.trim().is_empty() {
            return Err("external location must not be empty");
        }
        Ok(Self(location))
    }

    /// Returns the opaque location text.
    ///
    /// The adapter-defined location reference.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identifies a deterministic rendering implementation version.
impl RendererVersion {
    /// Creates a renderer version label.
    ///
    /// - `version`: Stable renderer version label.
    ///
    /// The renderer version.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }
}

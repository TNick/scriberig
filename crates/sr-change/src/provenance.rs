//! Defines source and provenance metadata.

use std::collections::BTreeMap;

/// Identifies the universal source of a change.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceKind {
    /// Human-authored change.
    User,
    /// Change observed from a projection.
    ExternalProjection,
    /// Change produced by an agent.
    Agent,
    /// Change requested through the CLI.
    Cli,
    /// Change requested through an API.
    Api,
    /// Change requested through the web editor.
    WebEditor,
    /// Change requested through the TUI.
    Tui,
    /// Change imported from another representation.
    Importer,
    /// Change produced by refactoring.
    Refactoring,
    /// Change produced by a merge.
    Merge,
    /// Change produced by synchronization.
    Synchronization,
    /// System-generated change.
    System,
    /// Extensible source category.
    Other(String),
}

/// Carries generic change provenance without requiring agent-specific fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    /// Universal source category.
    pub source: SourceKind,
    /// Optional actor or tool identifier.
    pub actor: Option<String>,
    /// Optional branch or workspace reference.
    pub workspace: Option<String>,
    /// Optional projection reference.
    pub projection: Option<String>,
    /// Optional validation evidence references.
    pub validation: Vec<String>,
    /// Optional parent migration references.
    pub parents: Vec<String>,
    /// Extensible deterministic metadata.
    pub metadata: BTreeMap<String, String>,
}

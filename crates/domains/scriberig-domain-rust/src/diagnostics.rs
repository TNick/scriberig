//! Defines derived Rust compiler and lint diagnostics.

use scriberig_core::EntityId;

/// Severity of a derived compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    /// Informational compiler note.
    Note,
    /// Warning that does not prevent analysis.
    Warning,
    /// Error that prevented a fact from being resolved.
    Error,
}

/// A diagnostic retained as derived knowledge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustDiagnostic {
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Stable analyzer code when available.
    pub code: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Canonical entity affected by the diagnostic.
    pub entity: Option<EntityId>,
}

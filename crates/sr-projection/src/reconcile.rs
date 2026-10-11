//! Defines typed outcomes for external representation reconciliation.

/// Represents semantic reconciliation without encoding status in text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome<Migration, Conflict, Detail> {
    /// External changes have no canonical semantic effect.
    NoSemanticChange,
    /// Reconciliation produced a proposed semantic migration.
    MigrationProposal(Migration),
    /// Available evidence cannot establish one semantic interpretation.
    Ambiguous(Detail),
    /// Reconciliation found an incompatible semantic change.
    Conflict(Conflict),
    /// The external representation violates domain validity rules.
    InvalidExternalState(Detail),
}

//! Defines provenance-bearing validation evidence and conservative reuse.

use scriberig_core::{EntityRevisionRef, Provenance, SnapshotId};

/// Outcome of a validator, kept distinct from authorization or promotion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationStatus {
    /// Validation completed successfully.
    Passed,
    /// Validation completed and found diagnostics.
    Failed,
    /// Validator could not complete.
    Error,
    /// Validator intentionally did not run.
    Skipped,
    /// Outcome is not known.
    Unknown,
}

/// Structured severity for one validation diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    /// Informational diagnostic.
    Info,
    /// Warning diagnostic.
    Warning,
    /// Error diagnostic.
    Error,
}

/// One structured validator diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationDiagnostic {
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Stable category code.
    pub category: String,
    /// Human-readable message.
    pub message: String,
    /// Optional exact subject reference.
    pub subject: Option<EntityRevisionRef>,
    /// Optional source coordinates represented as an opaque label.
    pub source: Option<String>,
}

/// Immutable validation evidence bound to exact candidate inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationEvidence {
    /// Validator identity.
    pub validator: String,
    /// Validator version/profile/configuration identity.
    pub configuration: String,
    /// Candidate snapshot, when validation was snapshot-scoped.
    pub candidate: Option<SnapshotId>,
    /// Exact immutable dependencies inspected by the validator.
    pub dependencies: Option<Vec<EntityRevisionRef>>,
    /// Structured outcome.
    pub status: ValidationStatus,
    /// Structured diagnostics.
    pub diagnostics: Vec<ValidationDiagnostic>,
    /// Produced artifact references.
    pub artifacts: Vec<EntityRevisionRef>,
    /// Origin and exact inputs for the evidence itself.
    pub provenance: Provenance,
}

/// Returns whether evidence can be reused for a later candidate.
///
/// Evidence is reusable only when validator configuration, candidate snapshot,
/// and complete dependency identities all match exactly.
///
/// # Parameters
/// - `evidence`: Prior validation evidence.
/// - `candidate`: Candidate snapshot being considered.
/// - `dependencies`: Exact dependencies observed for the candidate.
///
/// # Returns
/// `true` only when reuse is proven safe.
#[must_use]
pub fn evidence_applicable(
    evidence: &ValidationEvidence,
    candidate: Option<SnapshotId>,
    dependencies: Option<&[EntityRevisionRef]>,
) -> bool {
    evidence.candidate == candidate
        && evidence.dependencies.as_deref() == dependencies
        && evidence.dependencies.is_some()
}

/// Supplies a candidate to an application-level validator.
pub trait Validator {
    /// Validator-specific error type.
    type Error;
    /// Validates one explicit candidate without promoting it.
    ///
    /// # Parameters
    /// - `candidate`: Explicit snapshot and dependency context.
    ///
    /// # Returns
    /// Structured validation evidence.
    ///
    /// # Errors
    /// Returns the validator-specific execution error.
    fn validate(&self, candidate: ValidationCandidate) -> Result<ValidationEvidence, Self::Error>;
}

/// Explicit candidate context supplied to a validator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationCandidate {
    /// Candidate snapshot identity.
    pub snapshot: Option<SnapshotId>,
    /// Exact dependencies available to the validator.
    pub dependencies: Option<Vec<EntityRevisionRef>>,
}

#[cfg(test)]
mod tests {
    use super::{ValidationEvidence, ValidationStatus, evidence_applicable};
    use scriberig_core::{
        EntityId, EntityRevisionRef, Provenance, ProvenanceSource, RevisionId, SnapshotId,
    };

    /// Confirms evidence reuse is conservative and exact.
    #[test]
    fn evidence_requires_exact_dependencies() {
        let dependency = EntityRevisionRef {
            entity_id: EntityId::new([1; 16]),
            revision_id: RevisionId::new([2; 32]),
        };
        let evidence = ValidationEvidence {
            validator: "check".to_owned(),
            configuration: "v1".to_owned(),
            candidate: Some(SnapshotId::new([3; 32])),
            dependencies: Some(vec![dependency]),
            status: ValidationStatus::Passed,
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
            provenance: Provenance {
                source: ProvenanceSource::System,
                producer: None,
                producer_version: None,
                inputs: vec![dependency],
                profile: None,
                note: None,
            },
        };
        assert!(evidence_applicable(
            &evidence,
            evidence.candidate,
            Some(&[dependency])
        ));
        assert!(!evidence_applicable(&evidence, evidence.candidate, None));
        assert!(!evidence_applicable(
            &evidence,
            Some(SnapshotId::new([4; 32])),
            Some(&[dependency])
        ));
    }
}

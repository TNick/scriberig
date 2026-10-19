//! Defines immutable projection evidence, conflicts, and resolution inputs.

use scriberig_core::{EntityId, Provenance, RevisionId, SnapshotId};

use crate::model::{ProjectionRef, ProjectionSpecRef};

/// Stable identity for one captured external observation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObservationId(pub EntityId);

/// Stable identity for one immutable projection conflict.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConflictId(pub EntityId);

/// Immutable external content captured before reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedObservation {
    /// Stable observation identity.
    pub id: ObservationId,
    /// Projection instance that changed.
    pub projection: ProjectionRef,
    /// Previously rendered canonical revision, when available.
    pub previous_rendered_revision: Option<RevisionId>,
    /// Exact externally observed bytes.
    pub observed_content: Vec<u8>,
    /// Optional deterministic observation metadata supplied by the adapter.
    pub metadata: Vec<(String, String)>,
    /// Canonical semantic snapshot current at capture time.
    pub semantic_snapshot: SnapshotId,
    /// Parser or adapter identity.
    pub adapter_id: String,
    /// Parser or adapter version.
    pub adapter_version: String,
}

/// A pair of candidate mappings that could explain an observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmbiguousMapping {
    /// Entity involved in the ambiguity.
    pub entity: EntityId,
    /// Candidate revisions in deterministic order.
    pub candidates: Vec<RevisionId>,
}

/// Classifies why reconciliation cannot safely produce a migration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictKind {
    /// Multiple semantic mappings fit the observed content.
    AmbiguousMapping,
    /// Canonical state changed concurrently with the observation.
    ConcurrentCanonicalChange,
    /// The adapter could not parse the captured content.
    ParseFailure,
}

/// Immutable conflict lifecycle state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictStatus {
    /// No interpretation has been accepted.
    Unresolved,
    /// A resolution operation accepted an interpretation.
    Resolved {
        /// Stable resolution operation identity.
        resolution: EntityId,
    },
}

/// Structured projection conflict retaining both canonical and external state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionConflict {
    /// Stable conflict identity.
    pub id: ConflictId,
    /// Projection specification that owns the conflict.
    pub specification: ProjectionSpecRef,
    /// Projection instance that produced the observation.
    pub projection: ProjectionRef,
    /// Canonical snapshot used as the reconciliation base.
    pub base_snapshot: SnapshotId,
    /// Canonical snapshot current when the conflict was detected.
    pub current_snapshot: SnapshotId,
    /// Immutable external evidence that triggered the conflict.
    pub observation: CapturedObservation,
    /// Candidate parsed revision, when parsing produced one.
    pub candidate_revision: Option<RevisionId>,
    /// Ambiguous entity mappings, if any.
    pub ambiguous_mappings: Vec<AmbiguousMapping>,
    /// Entities participating in concurrent conflicts.
    pub conflicting_entities: Vec<EntityId>,
    /// Conflict classification.
    pub kind: ConflictKind,
    /// Immutable lifecycle status.
    pub status: ConflictStatus,
}

/// Exact interpretation selected to resolve one conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictResolution {
    /// Conflict being resolved.
    pub conflict: ConflictId,
    /// Selected candidate revision, when applicable.
    pub selected_revision: Option<RevisionId>,
    /// Adapter-defined interpretation details.
    pub interpretation: String,
}

/// Result envelope for an application-level resolution operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedConflict<Migration> {
    /// Resolution input that produced the migration candidate.
    pub resolution: ConflictResolution,
    /// Normal migration or candidate produced by the caller's validation path.
    pub migration: Migration,
    /// Provenance linking the candidate to exact conflict evidence.
    pub provenance: Provenance,
}

/// Persistence boundary for append-only projection evidence and conflicts.
pub trait ProjectionConflictStore {
    /// Stores an immutable external observation.
    ///
    /// - `observation`: Exact captured evidence.
    ///
    /// Unit after the evidence is recorded.
    ///
    /// Returns a backend-defined error when persistence fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn record_observation(&mut self, observation: CapturedObservation) -> Result<(), String>;

    /// Stores an immutable unresolved or resolved conflict record.
    ///
    /// - `conflict`: Structured conflict to append.
    ///
    /// Unit after the conflict is recorded.
    ///
    /// Returns a backend-defined error when persistence fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn record_conflict(&mut self, conflict: ProjectionConflict) -> Result<(), String>;
}

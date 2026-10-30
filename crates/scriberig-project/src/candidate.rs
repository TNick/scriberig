//! Implements shared project source candidates and safe application.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::ProjectCandidateRecord;

use crate::{ProjectError, ProjectService};

#[path = "candidate_support.rs"]
mod candidate_support;
use candidate_support::{candidate_id, validate_project};

#[path = "candidate_validation_digest.rs"]
mod validation_digest;
pub(crate) use validation_digest::candidate_validation_digest;

/// Loads source files visible in one exact candidate base snapshot.
///
/// - `store`: Shared project storage.
/// - `project`: Owning project.
/// - `snapshot`: Exact source snapshot.
///
/// # Returns
/// Deterministically ordered source records.
///
/// # Errors
/// Returns an error when snapshot source evidence is inconsistent.
pub(crate) fn project_files_at(
    store: &scriberig_store_sqlite::SqliteStore,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> Result<Vec<scriberig_store_sqlite::ProjectFileRecord>, ProjectError> {
    base::project_files_at(store, project, snapshot)
}

#[path = "candidate_aggregate.rs"]
mod aggregate;
#[path = "candidate_apply.rs"]
mod apply;
#[path = "candidate_apply_scope.rs"]
mod apply_scope;
#[path = "candidate_base.rs"]
mod base;
#[path = "candidate_merge.rs"]
mod merge;
#[path = "candidate_persistence.rs"]
mod persistence;
#[path = "candidate_validation.rs"]
mod validation;
#[path = "candidate_validation_lock.rs"]
mod validation_lock;
#[path = "candidate_validation_profiles.rs"]
mod validation_profiles;
pub use validation_profiles::CandidateValidationProfile;

/// Validation state for a source candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateValidation {
    /// Candidate has not yet been validated in its complete project state.
    Pending,
    /// Semantic and project validation passed.
    Passed,
    /// Semantic parsing passed but project validation failed.
    Failed {
        /// Bounded project validation diagnostics.
        diagnostics: Vec<String>,
    },
}

/// Summarizes durable aggregate and per-path state after candidate materialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaterializationReceipt {
    /// Candidate whose source effects were attempted.
    pub candidate: String,
    /// Durable aggregate effect state.
    pub state: String,
    /// Aggregate recovery diagnostic, when present.
    pub diagnostic: Option<String>,
    /// Durable receipts for each affected project-relative source path.
    pub paths: Vec<ProjectMaterializationPath>,
}

/// Describes the durable materialization state for one candidate path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaterializationPath {
    /// Exact project-relative source path.
    pub path: String,
    /// Durable state such as applied, failed, conflict, or not attempted.
    pub state: String,
    /// Safe path-level recovery diagnostic, when present.
    pub diagnostic: Option<String>,
}

/// Immutable source candidate proposed against an exact project snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCandidate {
    /// Stable candidate identity.
    pub id: String,
    /// Project receiving the candidate.
    pub project: ProjectId,
    /// Exact canonical base snapshot.
    pub base_snapshot: SnapshotId,
    /// Selected semantic entity.
    pub entity: EntityId,
    /// Stable semantic operation that created this candidate, when domain-known.
    pub semantic_operation: Option<String>,
    /// Exact semantic entity targeted by the operation, when domain-known.
    pub semantic_target: Option<EntityId>,
    /// Bounded typed semantic before/after changes for domain-aware review.
    pub semantic_changes: Vec<SemanticCandidateChange>,
    /// Project-relative source path.
    pub path: String,
    /// Source bytes at candidate creation.
    pub old: Vec<u8>,
    /// Proposed source bytes.
    pub new: Vec<u8>,
    /// Ordered source effects in this coherent candidate.
    pub effects: Vec<ProjectSourceEffect>,
    /// Structured validation result.
    pub validation: CandidateValidation,
    /// Most recent validation profile applied to this exact candidate.
    pub validation_profile: Option<String>,
    /// Exact aggregate output bytes accepted by the most recent passing validation.
    pub validation_digest: Option<[u8; 32]>,
}

/// Describes one typed semantic value changed by a durable candidate.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SemanticCandidateChange {
    /// Semantic entity whose value changed.
    pub target: EntityId,
    /// Stable domain property key, such as `cell.value`.
    pub property: String,
    /// Exact prior semantic value represented as JSON data.
    pub before: serde_json::Value,
    /// Proposed semantic value represented as JSON data.
    pub after: serde_json::Value,
}

/// Describes a project-relative source change and its exact base precondition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceEffect {
    /// Project-relative path; absolute paths are rejected.
    pub path: String,
    /// Kind of source change.
    pub kind: ProjectSourceEffectKind,
    /// Exact prior bytes for updates and deletes.
    pub old: Option<Vec<u8>>,
    /// Exact live bytes required before materialization when rebasing an external edit.
    ///
    /// `None` uses `old`; a value preserves the canonical base in `old` while
    /// independently binding the externally observed filesystem state.
    pub observed: Option<Vec<u8>>,
    /// Proposed bytes for creates and updates.
    pub new: Option<Vec<u8>>,
}

/// Supported source effect kinds for a project candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSourceEffectKind {
    /// Create a path absent from the exact base.
    Create,
    /// Replace a path owned by the exact base.
    Update,
    /// Remove a path owned by the exact base.
    Delete,
}

/// Describes one requested project-relative source change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceChange {
    /// Project-relative path to change.
    pub path: String,
    /// Requested source change kind.
    pub kind: ProjectSourceChangeKind,
    /// New source bytes, omitted only for delete.
    pub content: Option<Vec<u8>>,
}

/// Requested source change operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSourceChangeKind {
    /// Create an absent path.
    Create,
    /// Update an owned path.
    Update,
    /// Delete an owned path.
    Delete,
}

/// Result of applying a validated project candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateApplyReport {
    /// Candidate identity that was applied.
    pub candidate: String,
    /// Exact base snapshot required by the candidate.
    pub base_snapshot: SnapshotId,
    /// Successor snapshot after the project refresh.
    pub successor_snapshot: SnapshotId,
    /// Materialized project-relative path.
    pub path: String,
    /// All project-relative paths materialized by the aggregate candidate.
    pub paths: Vec<String>,
    /// Durable filesystem effect state.
    pub effect_state: String,
}

impl ProjectService {
    /// Creates a source candidate without changing canonical state or disk.
    ///
    /// - `project`: Project receiving the candidate.
    /// - `entity`: Selected semantic entity.
    /// - `text`: Complete edited source projection.
    ///
    /// An immutable candidate tied to the current project snapshot.
    ///
    /// Returns a structured error when the project, entity, or source is
    /// unavailable, or when the edited Rust cannot be reconciled.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn propose_candidate(
        &self,
        project: ProjectId,
        entity: EntityId,
        text: String,
    ) -> Result<ProjectCandidate, ProjectError> {
        let workspace = self.workspace(project)?;
        let entities = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?;
        let entity_record = entities
            .iter()
            .find(|record| record.entity == entity)
            .ok_or_else(|| ProjectError {
                code: "entity_not_found",
                message: "selected semantic entity is not owned by the project".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let file = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_error)?
            .into_iter()
            .find(|file| file.relative_path == entity_record.relative_path)
            .ok_or_else(|| ProjectError {
                code: "source_not_found",
                message: "selected entity has no source projection".to_owned(),
                diagnostics: Vec::new(),
            })?;
        crate::project_security::require_public_path(
            &self.project_path(project)?.join(&file.relative_path),
        )?;
        if file.adapter == "rust" {
            scriberig_domain_rust::named_symbols(&text).map_err(|error| ProjectError {
                code: "validation_failed",
                message: error,
                diagnostics: Vec::new(),
            })?;
        }
        let validation = validate_project(&self.project_path(project)?, &file.relative_path, &text);
        let id = candidate_id(project, workspace.snapshot, entity, &text);
        let new = text.into_bytes();
        let old = file.bytes;
        let path = file.relative_path;
        let effect = ProjectSourceEffect {
            path: path.clone(),
            kind: ProjectSourceEffectKind::Update,
            old: Some(old.clone()),
            observed: None,
            new: Some(new.clone()),
        };
        let validation_digest = matches!(&validation, super::CandidateValidation::Passed)
            .then(|| candidate_validation_digest(std::slice::from_ref(&effect)));
        let candidate = ProjectCandidate {
            id,
            project,
            base_snapshot: workspace.snapshot,
            entity,
            semantic_operation: None,
            semantic_target: None,
            semantic_changes: Vec::new(),
            path: path.clone(),
            old: old.clone(),
            new: new.clone(),
            effects: vec![effect],
            validation,
            validation_profile: None,
            validation_digest,
        };
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }

    /// Loads a candidate from durable project storage.
    ///
    /// - `id`: Candidate identity.
    ///
    /// The durable candidate when it exists.
    ///
    /// Returns a storage or corruption error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn candidate(&self, id: &str) -> Result<Option<ProjectCandidate>, ProjectError> {
        let record = self
            .store
            .project_candidate(id)
            .map_err(crate::errors::storage_error)?;
        let Some(record) = record else {
            return Ok(None);
        };
        crate::project_security::require_public_path(
            &self.project_path(record.project)?.join(&record.path),
        )?;
        Self::candidate_from_record(record).map(Some)
    }

    /// Reads durable aggregate and per-path materialization state for a candidate.
    ///
    /// # Parameters
    /// - `id`: Stable candidate identity.
    ///
    /// # Returns
    /// Recovery receipts when materialization has started, otherwise `None`.
    ///
    /// # Errors
    /// Returns storage, candidate decoding, or project path validation errors.
    pub fn materialization_receipt(
        &self,
        id: &str,
    ) -> Result<Option<ProjectMaterializationReceipt>, ProjectError> {
        if self.candidate(id)?.is_none() {
            return Ok(None);
        }
        let aggregate = self
            .store
            .project_effect(id)
            .map_err(crate::errors::storage_error)?;
        let paths = self
            .store
            .project_effect_paths(id)
            .map_err(crate::errors::storage_error)?;
        if aggregate.is_none() && paths.is_empty() {
            return Ok(None);
        }
        Ok(Some(ProjectMaterializationReceipt {
            candidate: id.to_owned(),
            state: aggregate
                .as_ref()
                .map_or_else(|| "unknown".to_owned(), |effect| effect.state.clone()),
            diagnostic: aggregate.and_then(|effect| effect.diagnostic),
            paths: paths
                .into_iter()
                .map(|receipt| ProjectMaterializationPath {
                    path: receipt.path,
                    state: receipt.state,
                    diagnostic: receipt.diagnostic,
                })
                .collect(),
        }))
    }

    /// Converts durable candidate storage data into a public candidate.
    ///
    /// - `record`: Durable candidate data.
    ///
    /// The reconstructed candidate.
    ///
    /// Returns a corruption error for invalid stored state.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn candidate_from_record(
        record: ProjectCandidateRecord,
    ) -> Result<ProjectCandidate, ProjectError> {
        ProjectCandidate::from_record(record)
    }
}

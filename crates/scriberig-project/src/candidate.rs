//! Implements shared project source candidates and safe application.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::ProjectCandidateRecord;

use crate::{ProjectError, ProjectService};

#[path = "candidate_support.rs"]
mod candidate_support;
use candidate_support::{candidate_id, validate_project};

#[path = "candidate_aggregate.rs"]
mod aggregate;
#[path = "candidate_apply.rs"]
mod apply;
#[path = "candidate_apply_scope.rs"]
mod apply_scope;
#[path = "candidate_base.rs"]
mod base;
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
    /// # Parameters
    /// - `project`: Project receiving the candidate.
    /// - `entity`: Selected semantic entity.
    /// - `text`: Complete edited source projection.
    ///
    /// # Returns
    /// An immutable candidate tied to the current project snapshot.
    ///
    /// # Errors
    /// Returns a structured error when the project, entity, or source is
    /// unavailable, or when the edited Rust cannot be reconciled.
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
        let candidate = ProjectCandidate {
            id,
            project,
            base_snapshot: workspace.snapshot,
            entity,
            path: path.clone(),
            old: old.clone(),
            new: new.clone(),
            effects: vec![ProjectSourceEffect {
                path,
                kind: ProjectSourceEffectKind::Update,
                old: Some(old),
                new: Some(new),
            }],
            validation,
            validation_profile: None,
        };
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }

    /// Loads a candidate from durable project storage.
    ///
    /// # Parameters
    /// - `id`: Candidate identity.
    ///
    /// # Returns
    /// The durable candidate when it exists.
    ///
    /// # Errors
    /// Returns a storage or corruption error.
    pub fn candidate(&self, id: &str) -> Result<Option<ProjectCandidate>, ProjectError> {
        self.store
            .project_candidate(id)
            .map_err(crate::errors::storage_error)?
            .map(Self::candidate_from_record)
            .transpose()
    }

    /// Converts durable candidate storage data into a public candidate.
    ///
    /// # Parameters
    /// - `record`: Durable candidate data.
    ///
    /// # Returns
    /// The reconstructed candidate.
    ///
    /// # Errors
    /// Returns a corruption error for invalid stored state.
    fn candidate_from_record(
        record: ProjectCandidateRecord,
    ) -> Result<ProjectCandidate, ProjectError> {
        ProjectCandidate::from_record(record)
    }
}

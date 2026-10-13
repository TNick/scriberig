//! Implements shared project source candidates and safe application.

use std::fs;
use std::path::PathBuf;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::{ProjectCandidateRecord, ProjectEffectRecord};
use sr_projection_fs::{FileSystem, LocalFileSystem};

use crate::{ProjectError, ProjectService};

#[path = "candidate_support.rs"]
mod candidate_support;
use candidate_support::{candidate_id, safe_join, validate_project};

#[path = "candidate_aggregate.rs"]
mod aggregate;
#[path = "candidate_persistence.rs"]
mod persistence;

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

    /// Applies a validated candidate using snapshot and filesystem CAS checks.
    ///
    /// # Parameters
    /// - `candidate`: Candidate to apply.
    ///
    /// # Returns
    /// The promoted successor snapshot and materialization receipt.
    ///
    /// # Errors
    /// Returns a stale, conflict, validation, filesystem, or import error.
    pub fn apply_candidate(
        &self,
        candidate: &ProjectCandidate,
    ) -> Result<CandidateApplyReport, ProjectError> {
        if let CandidateValidation::Pending = &candidate.validation {
            return Err(ProjectError {
                code: "validation_pending",
                message: "candidate requires exact-state validation before application".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        if let CandidateValidation::Failed { diagnostics } = &candidate.validation {
            return Err(ProjectError {
                code: "validation_failed",
                message: diagnostics.join("; "),
                diagnostics: Vec::new(),
            });
        }
        self.record_effect(candidate, None, "attempted", None)?;
        let workspace = self.workspace(candidate.project)?;
        if workspace.snapshot != candidate.base_snapshot {
            return Err(ProjectError {
                code: "stale_candidate",
                message: "project snapshot advanced since candidate creation".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        let root = self.project_path(candidate.project)?;
        let path = safe_join(&root, &candidate.path)?;
        let actual = fs::read(&path).map_err(|error| ProjectError {
            code: "filesystem_read_failed",
            message: error.to_string(),
            diagnostics: Vec::new(),
        })?;
        if actual != candidate.old {
            let error = ProjectError {
                code: "filesystem_conflict",
                message: "source changed externally since candidate creation".to_owned(),
                diagnostics: Vec::new(),
            };
            self.record_effect(candidate, None, "failed", Some(&error.message))?;
            return Err(error);
        }
        if let Err(error) = LocalFileSystem
            .write_atomic(&path, &candidate.new)
            .map_err(|error| ProjectError {
                code: "filesystem_write_failed",
                message: error.to_string(),
                diagnostics: Vec::new(),
            })
        {
            self.record_effect(candidate, None, "failed", Some(&error.message))?;
            return Err(error);
        }
        if let Err(error) = self.scan(Some(workspace.project.name.clone()), &root) {
            self.record_effect(candidate, None, "unknown", Some(&error.message))?;
            return Err(error);
        }
        let imported = match self.import(candidate.project) {
            Ok(imported) => imported,
            Err(error) => {
                self.record_effect(candidate, None, "unknown", Some(&error.message))?;
                return Err(error);
            }
        };
        self.record_effect(candidate, Some(imported.snapshot), "applied", None)?;
        Ok(CandidateApplyReport {
            candidate: candidate.id.clone(),
            base_snapshot: candidate.base_snapshot,
            successor_snapshot: imported.snapshot,
            path: candidate.path.clone(),
            effect_state: "applied".to_owned(),
        })
    }

    /// Records the durable state of a candidate's filesystem effect.
    ///
    /// # Parameters
    /// - `candidate`: Candidate whose effect is changing.
    /// - `successor`: Promoted snapshot, when known.
    /// - `state`: Durable effect state.
    /// - `diagnostic`: Safe failure or recovery detail.
    ///
    /// # Returns
    /// Unit after the receipt is persisted.
    ///
    /// # Errors
    /// Returns a storage error when the receipt cannot be persisted.
    fn record_effect(
        &self,
        candidate: &ProjectCandidate,
        successor: Option<SnapshotId>,
        state: &str,
        diagnostic: Option<&str>,
    ) -> Result<(), ProjectError> {
        self.store
            .put_project_effect(&ProjectEffectRecord {
                candidate: candidate.id.clone(),
                project: candidate.project,
                base_snapshot: candidate.base_snapshot,
                successor_snapshot: successor,
                path: candidate.path.clone(),
                state: state.to_owned(),
                diagnostic: diagnostic.map(ToOwned::to_owned),
            })
            .map_err(crate::errors::storage_error)
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

    /// Resolves one project's authorized filesystem root.
    ///
    /// # Parameters
    /// - `project`: Project identity.
    ///
    /// # Returns
    /// The registered project root.
    ///
    /// # Errors
    /// Returns an error when the project is unavailable or storage fails.
    fn project_path(&self, project: ProjectId) -> Result<PathBuf, ProjectError> {
        self.store
            .project(project)
            .map_err(crate::errors::storage_error)?
            .map(|record| record.path)
            .ok_or_else(|| ProjectError {
                code: "project_not_found",
                message: project.to_string(),
                diagnostics: Vec::new(),
            })
    }
}

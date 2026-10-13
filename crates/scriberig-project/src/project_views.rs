//! Exposes snapshot-bound project symbols and source projections.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{ProjectError, ProjectService};

/// A named semantic symbol owned by a project source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSymbol {
    /// Stable semantic entity identity.
    pub entity: EntityId,
    /// Exact snapshot containing the symbol.
    pub snapshot: SnapshotId,
    /// Project-relative source path.
    pub path: String,
    /// Semantic kind such as `rust.method`.
    pub kind: String,
    /// Human-readable semantic name when available.
    pub name: Option<String>,
}

/// An exact source projection associated with one semantic entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceProjection {
    /// Project identity owning the projection.
    pub project: ProjectId,
    /// Exact snapshot rendered by the projection.
    pub snapshot: SnapshotId,
    /// Selected semantic entity.
    pub entity: EntityId,
    /// Selected semantic name.
    pub name: Option<String>,
    /// Semantic kind.
    pub kind: String,
    /// Project-relative source path.
    pub path: String,
    /// Source bytes rendered by the projection.
    pub text: Vec<u8>,
    /// Exact BLAKE3 digest of the imported source bytes.
    pub source_digest: [u8; 32],
    /// Whether this semantic projection supports editing.
    pub editable: bool,
}

impl ProjectService {
    /// Lists named semantic symbols in the current project snapshot.
    ///
    /// # Parameters
    /// - `project`: Project whose symbols are requested.
    ///
    /// # Returns
    /// Snapshot-bound named symbols and source entities.
    ///
    /// # Errors
    /// Returns a structured error when the project or symbol store is unavailable.
    pub fn symbols(&self, project: ProjectId) -> Result<Vec<ProjectSymbol>, ProjectError> {
        let workspace = self.workspace(project)?;
        self.symbols_at(project, workspace.snapshot)
    }

    /// Lists project symbols only when the requested snapshot remains current.
    ///
    /// # Parameters
    /// - `project`: Project whose symbols are requested.
    /// - `snapshot`: Exact immutable snapshot scope.
    ///
    /// # Returns
    /// Snapshot-bound symbols and source entities.
    ///
    /// # Errors
    /// Returns a structured error when the snapshot is stale or the project
    /// symbol store is unavailable.
    pub fn symbols_at(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
    ) -> Result<Vec<ProjectSymbol>, ProjectError> {
        let workspace = self.workspace(project)?;
        ensure_snapshot(workspace.snapshot, snapshot)?;
        self.store
            .project_entities(project)
            .map_err(crate::errors::storage_error)
            .map(|entities| {
                entities
                    .into_iter()
                    .filter(|entity| entity.name.is_some() || entity.kind == "source")
                    .map(|entity| ProjectSymbol {
                        entity: entity.entity,
                        snapshot: workspace.snapshot,
                        path: entity.relative_path,
                        kind: entity.kind,
                        name: entity.name,
                    })
                    .collect()
            })
    }

    /// Finds a bounded set of semantic entities at an exact project snapshot.
    ///
    /// This uses a filtered ownership query so interactive discovery does not
    /// deserialize every entity in a large project for each search.
    ///
    /// # Parameters
    /// - `project`: Project whose symbols are requested.
    /// - `snapshot`: Exact immutable snapshot scope.
    /// - `query`: Case-insensitive path, kind, or name substring.
    /// - `limit`: Maximum number of symbols returned.
    ///
    /// # Returns
    /// Snapshot-bound matching symbols and source entities.
    ///
    /// # Errors
    /// Returns a structured error when the snapshot or symbol store is stale.
    pub fn find_symbols_at(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ProjectSymbol>, ProjectError> {
        let workspace = self.workspace(project)?;
        ensure_snapshot(workspace.snapshot, snapshot)?;
        self.store
            .find_project_entities(project, query, limit)
            .map_err(crate::errors::storage_error)
            .map(|entities| {
                entities
                    .into_iter()
                    .map(|entity| ProjectSymbol {
                        entity: entity.entity,
                        snapshot: workspace.snapshot,
                        path: entity.relative_path,
                        kind: entity.kind,
                        name: entity.name,
                    })
                    .collect()
            })
    }

    /// Renders an exact source projection for one semantic entity.
    ///
    /// # Parameters
    /// - `project`: Project owning the entity.
    /// - `entity`: Semantic entity to render.
    ///
    /// # Returns
    /// Snapshot-bound source projection.
    ///
    /// # Errors
    /// Returns a structured error when the entity or source file is missing.
    pub fn projection(
        &self,
        project: ProjectId,
        entity: EntityId,
    ) -> Result<ProjectSourceProjection, ProjectError> {
        let workspace = self.workspace(project)?;
        self.projection_at(project, workspace.snapshot, entity)
    }

    /// Renders source only when the requested snapshot remains current.
    ///
    /// # Parameters
    /// - `project`: Project owning the entity.
    /// - `snapshot`: Exact immutable snapshot scope.
    /// - `entity`: Semantic entity to render.
    ///
    /// # Returns
    /// Source projection bound to the requested snapshot.
    ///
    /// # Errors
    /// Returns a structured error when the snapshot is stale or source is absent.
    pub fn projection_at(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<ProjectSourceProjection, ProjectError> {
        let workspace = self.workspace(project)?;
        ensure_snapshot(workspace.snapshot, snapshot)?;
        let entity_record = self
            .store
            .project_entity(project, entity)
            .map_err(crate::errors::storage_error)?
            .ok_or_else(|| ProjectError {
                code: "entity_not_found",
                message: "semantic entity is not owned by the project".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let source_entity = if entity_record.kind == "source" {
            entity
        } else {
            super::support::source_entity(project, &entity_record.relative_path)
        };
        let revision_id = self
            .store
            .resolve(snapshot, source_entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| ProjectError {
                code: "source_not_found",
                message: "semantic entity is absent from the requested snapshot".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| ProjectError {
                code: "source_not_found",
                message: "semantic source revision is unavailable".to_owned(),
                diagnostics: Vec::new(),
            })?;
        if revision.object_type().as_str() != "project.source" {
            return Err(ProjectError {
                code: "not_a_source",
                message: "semantic entity does not resolve to project source bytes".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        let source_digest = *blake3::hash(revision.payload()).as_bytes();
        Ok(ProjectSourceProjection {
            project,
            snapshot: workspace.snapshot,
            entity,
            name: entity_record.name,
            kind: entity_record.kind.clone(),
            path: entity_record.relative_path,
            text: revision.payload().to_vec(),
            source_digest,
            editable: entity_record.kind == "rust.method",
        })
    }
}

/// Requires a project operation to retain its exact workspace snapshot.
///
/// # Parameters
/// - `current`: Authoritative current project snapshot.
/// - `requested`: Snapshot attached to the caller's semantic references.
///
/// # Returns
/// Unit when both snapshots match.
///
/// # Errors
/// Returns a stale-snapshot error instead of silently retargeting.
fn ensure_snapshot(current: SnapshotId, requested: SnapshotId) -> Result<(), ProjectError> {
    if current == requested {
        return Ok(());
    }
    Err(ProjectError {
        code: "stale_snapshot",
        message: "project source operation is bound to a superseded snapshot".to_owned(),
        diagnostics: Vec::new(),
    })
}

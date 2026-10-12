//! Exposes snapshot-bound project symbols and source projections.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

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
        let entity_record = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?
            .into_iter()
            .find(|record| record.entity == entity)
            .ok_or_else(|| ProjectError {
                code: "entity_not_found",
                message: "semantic entity is not owned by the project".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let file = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_error)?
            .into_iter()
            .find(|record| record.relative_path == entity_record.relative_path)
            .ok_or_else(|| ProjectError {
                code: "source_not_found",
                message: "semantic entity has no source projection".to_owned(),
                diagnostics: Vec::new(),
            })?;
        Ok(ProjectSourceProjection {
            project,
            snapshot: workspace.snapshot,
            entity,
            name: entity_record.name,
            kind: entity_record.kind.clone(),
            path: entity_record.relative_path,
            text: file.bytes,
            editable: entity_record.kind == "rust.method",
        })
    }
}

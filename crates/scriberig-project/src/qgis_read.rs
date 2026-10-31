//! Reads exact-snapshot QGIS project and linked layer-reference semantics.

use scriberig_core::{ProjectId, RevisionId, SnapshotId};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{ProjectError, ProjectService};

/// Returns one QGIS project model and its exact immutable semantic revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisProjectRead {
    /// Snapshot containing the requested project model.
    pub snapshot: SnapshotId,
    /// Exact semantic revision of the QGIS project model.
    pub revision: RevisionId,
    /// Provider-neutral project and linked layer references.
    pub project: scriberig_domain_qgis::QgisProject,
}

impl ProjectService {
    /// Reads one QGIS project and resolved layer references from an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Owning Scriberig project.
    /// - `path`: Exact project-relative `.qgs` or `.qgz` path.
    /// - `snapshot`: Immutable snapshot to inspect.
    ///
    /// # Returns
    /// Parsed QGIS semantics, resolved `GeoLayer` identities, and the exact revision.
    ///
    /// # Errors
    /// Returns errors when the project source or semantic revision is unavailable
    /// in the requested snapshot, or when its payload is malformed.
    pub fn qgis_project(
        &self,
        project: ProjectId,
        path: &str,
        snapshot: SnapshotId,
    ) -> Result<QgisProjectRead, ProjectError> {
        let entity = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_debug_error)?
            .into_iter()
            .find(|entity| entity.kind == "qgis.project" && entity.relative_path == path)
            .ok_or_else(|| qgis_error("qgis_project_not_found", "QGIS project was not imported"))?;
        let revision = self
            .store
            .resolve(snapshot, entity.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                qgis_error(
                    "qgis_project_not_in_snapshot",
                    "QGIS project is absent from the requested snapshot",
                )
            })?;
        let record = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                qgis_error(
                    "qgis_project_revision_missing",
                    "QGIS project revision is unavailable",
                )
            })?;
        let model: scriberig_domain_qgis::QgisProject = serde_json::from_slice(record.payload())
            .map_err(|_| {
                qgis_error(
                    "qgis_project_invalid",
                    "QGIS project semantics are malformed",
                )
            })?;
        Ok(QgisProjectRead {
            snapshot,
            revision,
            project: model,
        })
    }
}

/// Creates a safe QGIS semantic read error.
///
/// # Parameters
/// - `code`: Stable error category.
/// - `message`: Safe description without source bytes.
///
/// # Returns
/// Project error for an exact-snapshot QGIS read.
fn qgis_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}

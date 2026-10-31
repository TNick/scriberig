//! Imports persisted project evidence into immutable semantic snapshots.

use scriberig_core::ProjectId;

#[path = "import_service/reconciliation.rs"]
mod reconciliation;
pub(crate) use reconciliation::PreviousCadDrawing;
use scriberig_domain_cad::CadFormat;
use scriberig_store::{RootStore, Snapshot, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::errors::{missing_project, promotion_error, storage_error};
use crate::support::import_file;
use crate::{
    ImportReport, ProjectError, ProjectService, ProjectWorkspace, artifact, manifest,
    project_security, support,
};

impl ProjectService {
    /// Imports the persisted scan into an immutable project snapshot.
    ///
    /// - `project`: Stable project identity to import.
    ///
    /// Import counts and the promoted snapshot.
    ///
    /// Returns a structured validation, storage, or promotion failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn import(&self, project: ProjectId) -> Result<ImportReport, ProjectError> {
        let files = self.store.project_files(project).map_err(storage_error)?;
        self.import_file_records(project, &files)
    }

    /// Imports only when the latest persisted manifest matches its exact digest.
    ///
    /// - `project`: Project owning the persisted scan.
    /// - `manifest_id`: Digest returned by the scan manifest summary.
    ///
    /// Import counts and the promoted immutable snapshot.
    ///
    /// Returns a structured error when the project is missing, the manifest is
    /// stale, or importing the captured manifest fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn import_manifest(
        &self,
        project: ProjectId,
        manifest_id: &str,
    ) -> Result<ImportReport, ProjectError> {
        let files = self.store.project_files(project).map_err(storage_error)?;
        if manifest::digest(project, &files) != manifest_id {
            return Err(ProjectError {
                code: "manifest_changed",
                message: "persisted project manifest no longer matches the reviewed scan".into(),
                diagnostics: Vec::new(),
            });
        }
        self.import_file_records(project, &files)
    }

    /// Imports the exact file records captured by one persisted manifest read.
    ///
    /// - `project`: Project being imported.
    /// - `files`: Immutable records captured from one persisted manifest read.
    ///
    /// Import counts and the promoted immutable snapshot.
    ///
    /// Returns a structured validation, storage, or promotion failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn import_file_records(
        &self,
        project: ProjectId,
        files: &[ProjectFileRecord],
    ) -> Result<ImportReport, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        for file in files {
            project_security::require_public_path(&record.path.join(&file.relative_path))?;
        }
        if files.is_empty() {
            return Err(ProjectError {
                code: "scan_required",
                message: "scan the project before importing".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        let root = support::project_root(project)?;
        let expected = self.store.read_root(&root);
        let old_entities = self
            .store
            .project_entities(project)
            .map_err(storage_error)?;
        let previous_cad_entities = match record.current_snapshot {
            Some(snapshot) => self.previous_cad_entities(snapshot, &old_entities)?,
            None => std::collections::HashMap::new(),
        };
        let dwg_provider = self
            .cad_provider_registry
            .provider_for(&CadFormat::Dwg)
            .filter(|provider| {
                provider
                    .descriptor()
                    .formats
                    .iter()
                    .any(|capability| capability.format == CadFormat::Dwg && capability.read)
            });
        self.require_dwg_provider_continuity(
            record.current_snapshot,
            &old_entities,
            files,
            dwg_provider.as_deref(),
        )?;
        let base = expected
            .or(record.current_snapshot)
            .unwrap_or_else(|| Snapshot::empty().id());
        let mut revisions = Vec::new();
        let mut changes = old_entities
            .iter()
            .map(|entity| SnapshotChange::Remove(entity.entity))
            .collect::<Vec<_>>();
        let mut ownership = Vec::<ProjectEntityRecord>::new();
        for file in files.iter().filter(|file| file.status == "ready") {
            ownership.extend(import_file(
                project,
                file,
                files,
                &mut revisions,
                &mut changes,
                &previous_cad_entities,
                dwg_provider.as_deref(),
            )?);
        }
        artifact::append_artifact_revisions(
            project,
            files,
            &mut revisions,
            &mut changes,
            &mut ownership,
        )?;
        let snapshot = self
            .store
            .apply_project_import(
                project, root, expected, base, &revisions, &changes, &ownership,
            )
            .map_err(|error| promotion_error(format!("{error:?}")))?;
        let project_record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        Ok(ImportReport {
            project: project_record,
            files: files.iter().filter(|file| file.status == "ready").count(),
            entities: ownership.len(),
            snapshot,
        })
    }

    /// Resolves an imported project's current snapshot and semantic root.
    ///
    /// - `project`: Project whose active workspace should be resolved.
    ///
    /// The project record, current snapshot, and semantic root entity.
    ///
    /// Returns a structured error when the project is missing or not imported.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn workspace(&self, project: ProjectId) -> Result<ProjectWorkspace, ProjectError> {
        let record = self
            .store
            .project(project)
            .map_err(storage_error)?
            .ok_or_else(|| missing_project(project))?;
        let snapshot = self
            .store
            .read_root(&support::project_root(project)?)
            .or(record.current_snapshot)
            .ok_or_else(|| ProjectError {
                code: "project_not_imported",
                message: "project has no current semantic snapshot".to_owned(),
                diagnostics: Vec::new(),
            })?;
        let mut project_record = record;
        project_record.current_snapshot = Some(snapshot);
        let root_entity = self
            .store
            .project_source_root(project)
            .map_err(storage_error)?;
        let root_entity = root_entity
            .map(|entity| entity.entity)
            .ok_or_else(|| ProjectError {
                code: "project_root_unavailable",
                message: "project has no semantic source root".to_owned(),
                diagnostics: Vec::new(),
            })?;
        Ok(ProjectWorkspace {
            project: project_record,
            snapshot,
            root_entity,
        })
    }
}

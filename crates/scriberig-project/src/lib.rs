//! Provides the shared project scan and semantic onboarding service.

use std::path::Path;
use std::sync::{Arc, Mutex};

use scriberig_store_sqlite::SqliteStore;

mod artifact;
#[path = "artifact_service.rs"]
mod artifact_service;
pub use artifact_service::{ArtifactSynchronizationState, ArtifactSynchronizationStatus};
#[path = "support.rs"]
mod support;
pub use artifact::{
    ArtifactCapability, ArtifactExternalReference, ArtifactFidelity, ArtifactGroupingDiagnostic,
    ArtifactId, ArtifactKind, ArtifactManifest, ArtifactMemberDescriptor, ArtifactMemberKey,
    ArtifactMemberPage, ArtifactSourceEvidence, OpaquePreservation, cad_provider_artifact_fidelity,
};
mod cad_bounds;
mod cad_find;
mod cad_query;
mod cad_read;
#[path = "candidate.rs"]
mod candidate;
mod document_find;
mod document_read;
mod docx_candidate;
mod docx_candidate_helpers;
mod docx_source_reconciliation;
mod dxf_candidate;
mod dxf_candidate_errors;
mod dxf_candidate_semantic;
mod geopackage_attribute_candidate;
mod geopackage_candidate;
mod geopackage_query;
mod geopackage_read;
#[path = "import_service.rs"]
mod import_service;
#[path = "manifest.rs"]
mod manifest;
mod project_security;
#[path = "project_views.rs"]
mod project_views;
mod qgis_candidate;
mod qgis_read;
#[path = "scan_manifest.rs"]
mod scan_manifest;
mod source_watch;
mod spreadsheet_read;
mod types;
mod xlsx_candidate;
mod xlsx_rename_candidate;
pub use cad_bounds::{CadBounds2D, CadBoundsEntityPage, CadBoundsQueryRequest};
pub use cad_find::{CadTextMatch, CadTextSearch, MAX_CAD_SEARCH_PAGE};
pub use cad_read::{CadDrawingView, CadEntityPage, MAX_CAD_ENTITY_PAGE};
pub use candidate::{
    CandidateApplyReport, CandidateValidation, CandidateValidationProfile, ProjectCandidate,
    ProjectMaterializationPath, ProjectMaterializationReceipt, ProjectSourceChange,
    ProjectSourceChangeKind, ProjectSourceEffect, ProjectSourceEffectKind, SemanticCandidateChange,
};
pub use document_find::{
    DocumentParagraphMatch, DocumentParagraphSearch, MAX_DOCUMENT_SEARCH_PAGE,
};
pub use document_read::{
    DocumentBlockView, DocumentCellView, DocumentRowView, DocumentSectionView, DocumentStyleView,
    DocumentTableView, DocumentTreeView,
};
pub use geopackage_attribute_candidate::GeoPackageFeatureAttributeEdit;
pub use geopackage_query::{
    GeoPackageFeatureQuery, GeoPackageFeatureQueryPage, GeoQueryProvenance, GeoQueryStrategy,
};
pub use geopackage_read::{GeoPackageCatalog, GeoPackageLayerSummary, MAX_GEOPACKAGE_LAYER_PAGE};
pub use manifest::{ProjectManifestEntry, ProjectManifestPage, ProjectManifestSummary};
pub use project_views::{ProjectSourceProjection, ProjectSymbol};
pub use qgis_read::QgisProjectRead;
pub use source_watch::{ProjectSourceObservation, ProjectSourceObservationResult};
pub use spreadsheet_read::{
    MAX_SPREADSHEET_CELL_PAGE, MAX_SPREADSHEET_SEARCH_PAGE, SpreadsheetCellMatch,
    SpreadsheetCellPage, SpreadsheetCellSearch, SpreadsheetWorkbookSummary,
    SpreadsheetWorksheetSummary,
};
pub use types::{
    ImportDiagnostic, ImportReport, ProjectError, ProjectFileRename, ProjectWorkspace, ScanFile,
    ScanOptions, ScanReport,
};
#[path = "errors.rs"]
mod errors;
use errors::{git_error, storage_error};

/// Shared project onboarding service used by CLI, HTTP, and TUI surfaces.
#[derive(Clone, Debug)]
pub struct ProjectService {
    /// Shared durable SQLite catalog and object store.
    store: Arc<SqliteStore>,
    /// Shared exact-digest source watcher coordination state.
    source_watch: Arc<Mutex<source_watch::ProjectSourceWatchCoordinator>>,
    /// Explicit host-injected providers for optional external CAD formats.
    cad_provider_registry: scriberig_domain_cad::CadProviderRegistry,
}

impl ProjectService {
    /// Creates a project service over a shared SQLite store.
    ///
    /// - `store`: Shared durable project and object store.
    ///
    /// A project onboarding service.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self {
            store,
            source_watch: Arc::new(Mutex::new(
                source_watch::ProjectSourceWatchCoordinator::default(),
            )),
            cad_provider_registry: scriberig_domain_cad::CadProviderRegistry::new(),
        }
    }

    /// Configures host-injected providers for optional CAD project imports.
    ///
    /// # Parameters
    /// - `registry`: Initialized providers and safe initialization failure states.
    ///
    /// # Returns
    /// The project service configured to use this registry during imports.
    #[must_use]
    pub fn with_cad_provider_registry(
        mut self,
        registry: scriberig_domain_cad::CadProviderRegistry,
    ) -> Self {
        self.cad_provider_registry = registry;
        self
    }

    /// Classifies a watcher event against committed candidate output receipts.
    ///
    /// Candidate application holds this coordinator across filesystem writes
    /// and project refresh. Concurrent event handlers wait for exact receipts
    /// before deciding whether a write should be re-imported.
    ///
    /// # Parameters
    /// - `project`: Project owning the observed source.
    /// - `path`: Project-relative source path reported by the watcher.
    /// - `observed_digest`: Exact current file digest, or `None` when absent.
    ///
    /// # Returns
    /// Whether the event is a candidate self-write or an external change.
    ///
    /// # Errors
    /// Returns an error if the live source watcher coordinator was poisoned.
    pub fn classify_source_observation(
        &self,
        project: scriberig_core::ProjectId,
        path: &str,
        observed_digest: Option<[u8; 32]>,
    ) -> Result<ProjectSourceObservation, ProjectError> {
        let mut coordinator = self.source_watch.lock().map_err(|_| ProjectError {
            code: "source_watch_unavailable",
            message: "project source watcher coordinator is unavailable".to_owned(),
            diagnostics: Vec::new(),
        })?;
        Ok(coordinator.classify(project, path, observed_digest))
    }

    /// Serializes a source event with candidate writes and imports external changes.
    ///
    /// The watcher should call this method for source events. Candidate writes
    /// and the scan/import for external events share one lock, so a candidate
    /// cannot pass its base-snapshot check while a watcher import is in flight.
    ///
    /// # Parameters
    /// - `project`: Project owning the observed source.
    /// - `path`: Project-relative source path reported by the watcher.
    /// - `observed_digest`: Exact observed file digest, or `None` when absent.
    ///
    /// # Returns
    /// The classification and the new snapshot for an imported external change.
    ///
    /// # Errors
    /// Returns a structured error when coordination, scan, or import fails.
    pub fn handle_source_observation(
        &self,
        project: scriberig_core::ProjectId,
        path: &str,
        observed_digest: Option<[u8; 32]>,
    ) -> Result<ProjectSourceObservationResult, ProjectError> {
        let mut coordinator = self.source_watch.lock().map_err(|_| ProjectError {
            code: "source_watch_unavailable",
            message: "project source watcher coordinator is unavailable".to_owned(),
            diagnostics: Vec::new(),
        })?;
        let observation = coordinator.classify(project, path, observed_digest);
        if observation == ProjectSourceObservation::SelfWrite {
            return Ok(ProjectSourceObservationResult {
                observation,
                imported_snapshot: None,
            });
        }
        if let Some(digest) = observed_digest {
            if let Some(candidate) =
                self.reconcile_external_docx_observation(project, path, digest)?
            {
                let validated = self.validate_candidate_profile_cancellable(
                    &candidate.id,
                    crate::CandidateValidationProfile::DocxPackage,
                    &std::sync::atomic::AtomicBool::new(false),
                )?;
                let applied = self.apply_candidate_with_coordinator(
                    &validated,
                    &sr_projection_fs::LocalFileSystem,
                    &mut coordinator,
                )?;
                return Ok(ProjectSourceObservationResult {
                    observation,
                    imported_snapshot: Some(applied.successor_snapshot),
                });
            }
        }
        let workspace = self.workspace(project)?;
        self.scan(
            Some(workspace.project.name.clone()),
            &workspace.project.path,
        )?;
        let imported = self.import(project)?;
        Ok(ProjectSourceObservationResult {
            observation,
            imported_snapshot: Some(imported.snapshot),
        })
    }

    /// Registers and persists a deterministic filesystem scan.
    ///
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project path.
    ///
    /// The project record and persisted scan manifest.
    ///
    /// Returns a structured filesystem or storage failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn scan(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
    ) -> Result<ScanReport, ProjectError> {
        self.scan_with_options(name, path, ScanOptions::default())
    }

    /// Registers and persists a scan using an explicit file-selection policy.
    ///
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project path.
    /// - `options`: File-selection policy for the persisted manifest.
    ///
    /// The registered project and selected deterministic scan files.
    ///
    /// Returns a structured filesystem, Git, or storage failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn scan_with_options(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
        options: ScanOptions,
    ) -> Result<ScanReport, ProjectError> {
        self.scan_with_renames(name, path, options, &[])
    }

    /// Scans a project and carries source identities across explicit file renames.
    ///
    /// Rename evidence applies only when the old path exists in the previous
    /// persisted scan, the old path is absent from the new scan, and the new
    /// path is newly present with the same adapter. This allows content to
    /// change during the rename without guessing identity from filename or
    /// similarity.
    ///
    /// # Parameters
    /// - `name`: Optional display name for a new project.
    /// - `path`: Filesystem project path.
    /// - `options`: File-selection policy for the persisted manifest.
    /// - `renames`: Explicit previous-path to current-path observations.
    ///
    /// # Returns
    /// The registered project and selected deterministic scan files.
    ///
    /// # Errors
    /// Returns a structured error for invalid rename evidence or filesystem,
    /// Git, or storage failures.
    pub fn scan_with_renames(
        &self,
        name: Option<String>,
        path: impl AsRef<Path>,
        options: ScanOptions,
        renames: &[ProjectFileRename],
    ) -> Result<ScanReport, ProjectError> {
        let path = path.as_ref();
        let display_name = name
            .or_else(|| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "project".to_owned());
        let project = self
            .store
            .register_project(display_name, path)
            .map_err(storage_error)?;
        let tracked = if options.git_tracked {
            Some(support::git_tracked_paths(&project.path).map_err(|error| git_error(&error))?)
        } else {
            None
        };
        self.persist_scan(&project, tracked.as_ref(), options.git_tracked, renames)
    }
}

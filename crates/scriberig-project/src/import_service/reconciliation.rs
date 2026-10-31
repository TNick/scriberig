//! Resolves prior CAD semantics for safe provider re-import reconciliation.

use std::collections::HashMap;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_domain_cad::CadExternalProvider;
use scriberig_domain_dxf::DxfPreviousEntity;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::{ProjectError, ProjectService};

/// Carries provider lineage and normalized semantics from a prior DWG import.
///
/// Public members: none.
/// Private members: provider identity and prior CAD entities.
pub(crate) struct PreviousCadDrawing {
    /// Provider identity recorded in the prior drawing summary.
    pub(crate) provider_id: Option<String>,
    /// Provider version recorded in the prior drawing summary.
    pub(crate) provider_version: Option<String>,
    /// Prior provider-neutral semantic entities and detached native handles.
    pub(crate) entities: Vec<DxfPreviousEntity>,
}

impl ProjectService {
    /// Loads prior CAD entities and provider lineage for files being re-imported.
    ///
    /// - `snapshot`: Exact immutable snapshot currently associated with the project.
    /// - `ownership`: Semantic ownership records from the project catalog.
    ///
    /// Prior CAD state indexed by project-relative source path.
    ///
    /// Returns a structured storage error when prior revisions cannot be loaded.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the snapshot store fails or prior CAD metadata is invalid.
    pub(super) fn previous_cad_entities(
        &self,
        snapshot: SnapshotId,
        ownership: &[ProjectEntityRecord],
    ) -> Result<HashMap<String, PreviousCadDrawing>, ProjectError> {
        let mut previous = HashMap::new();
        for drawing in ownership.iter().filter(|record| {
            record.kind == "cad.drawing"
                && record.relative_path.to_ascii_lowercase().ends_with(".dwg")
        }) {
            let Some(revision) = self
                .store
                .resolve(snapshot, drawing.entity)
                .map_err(crate::errors::storage_debug_error)?
                .map(|id| {
                    self.store
                        .get(id)
                        .map_err(crate::errors::storage_debug_error)
                })
                .transpose()?
                .flatten()
            else {
                continue;
            };
            let metadata: serde_json::Value =
                serde_json::from_slice(revision.payload()).unwrap_or(serde_json::Value::Null);
            let provider_id = metadata
                .get("provider_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let provider_version = metadata
                .get("provider_version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let mut entities = Vec::new();
            for record in ownership.iter().filter(|record| {
                record.relative_path == drawing.relative_path && record.kind == "cad.entity"
            }) {
                let Some(revision) = self
                    .store
                    .resolve(snapshot, record.entity)
                    .map_err(crate::errors::storage_debug_error)?
                    .map(|id| {
                        self.store
                            .get(id)
                            .map_err(crate::errors::storage_debug_error)
                    })
                    .transpose()?
                    .flatten()
                else {
                    continue;
                };
                let Ok(semantic) = serde_json::from_slice(revision.payload()) else {
                    continue;
                };
                let entity_key = EntityId::to_string(&record.entity);
                let handle = metadata
                    .get("entity_identifiers")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .find(|identifier| {
                        identifier.get("entity").and_then(serde_json::Value::as_str)
                            == Some(entity_key.as_str())
                    })
                    .and_then(|identifier| identifier.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                entities.push(DxfPreviousEntity {
                    semantic,
                    native_handle: handle,
                });
            }
            previous.insert(
                drawing.relative_path.clone(),
                PreviousCadDrawing {
                    provider_id,
                    provider_version,
                    entities,
                },
            );
        }
        Ok(previous)
    }

    /// Prevents re-import from silently removing CAD semantics when a provider is unavailable.
    ///
    /// - `snapshot`: Prior project snapshot, if the project was already imported.
    /// - `ownership`: Current semantic project ownership records.
    /// - `files`: Exact scan manifest being imported.
    /// - `provider`: Host-injected DWG provider selected for this import, if available.
    ///
    /// Unit when provider continuity is safe or no prior DWG semantics need it.
    ///
    /// Returns a structured diagnostic when provider-derived CAD state would be dropped.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when prior project state cannot be read or continuity is unsafe.
    pub(super) fn require_dwg_provider_continuity(
        &self,
        snapshot: Option<SnapshotId>,
        ownership: &[ProjectEntityRecord],
        files: &[ProjectFileRecord],
        provider: Option<&dyn CadExternalProvider>,
    ) -> Result<(), ProjectError> {
        let Some(snapshot) = snapshot else {
            return Ok(());
        };
        let current_dwg_paths = files
            .iter()
            .filter(|file| file.adapter == "dwg" && file.status == "ready")
            .map(|file| file.relative_path.as_str())
            .collect::<std::collections::HashSet<_>>();
        for record in ownership.iter().filter(|record| {
            record.kind == "cad.drawing"
                && record.relative_path.to_ascii_lowercase().ends_with(".dwg")
                && !current_dwg_paths.contains(record.relative_path.as_str())
        }) {
            let Some(revision) = self
                .store
                .resolve(snapshot, record.entity)
                .map_err(crate::errors::storage_debug_error)?
                .map(|id| {
                    self.store
                        .get(id)
                        .map_err(crate::errors::storage_debug_error)
                })
                .transpose()?
                .flatten()
            else {
                continue;
            };
            let metadata: serde_json::Value =
                serde_json::from_slice(revision.payload()).unwrap_or(serde_json::Value::Null);
            if metadata
                .get("provider_id")
                .and_then(serde_json::Value::as_str)
                .is_some()
            {
                return Err(ProjectError {
                    code: "dwg_provider_required",
                    message: "re-import would remove provider-derived CAD state without an explicit replacement".into(),
                    diagnostics: Vec::new(),
                });
            }
        }
        if provider.is_none()
            && ownership.iter().any(|record| {
                record.kind == "cad.drawing"
                    && record.relative_path.to_ascii_lowercase().ends_with(".dwg")
                    && current_dwg_paths.contains(record.relative_path.as_str())
            })
        {
            return Err(ProjectError {
                code: "dwg_provider_required",
                message: "a DWG provider is required to preserve previously imported CAD semantics"
                    .into(),
                diagnostics: Vec::new(),
            });
        }
        Ok(())
    }
}

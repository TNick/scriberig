//! Defines snapshot-aware application inspection requests and results.

use crate::{ApplicationContext, ApplicationError, ArtifactManifestPage, TrustSummary};
use scriberig_core::{EntityId, ObjectType, ProjectId, RevisionId, SnapshotId};

/// Selects how an application query resolves its snapshot context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotResolution {
    /// Use the application's current accepted snapshot.
    Current,
    /// Resolve the entity under this exact snapshot.
    At(SnapshotId),
    /// Read this exact revision, retaining an optional application snapshot.
    Revision(RevisionId),
}

/// Requests the compact or expanded inspection of one entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectRequest {
    /// Entity identity to inspect.
    pub entity: EntityId,
    /// Snapshot or exact revision resolution requested by the caller.
    pub snapshot: SnapshotResolution,
    /// Amount of semantic detail requested.
    pub detail: DetailLevel,
}

/// Requests a bounded spreadsheet cell page at an exact immutable snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellsQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact snapshot whose semantic cell revisions are read.
    pub snapshot: SnapshotId,
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// Optional inclusive coordinate bounds.
    pub range: Option<scriberig_domain_spreadsheet::CellRange>,
    /// Zero-based sparse-cell page offset.
    pub offset: usize,
    /// Maximum number of cells to return.
    pub limit: usize,
}

/// Requests inspection of one exact worksheet coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact snapshot whose cell revision is read.
    pub snapshot: SnapshotId,
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
}

/// Requests a shared candidate edit to one exact spreadsheet coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellEditQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable base snapshot.
    pub snapshot: SnapshotId,
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
    /// Authored scalar value or formula.
    pub value: scriberig_domain_spreadsheet::CellValue,
}

/// Requests one exact-snapshot semantic document outline and content tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentTreeQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable snapshot whose semantic records are read.
    pub snapshot: SnapshotId,
    /// Stable semantic document root identity.
    pub document: EntityId,
}

/// Requests a shared paragraph replacement candidate at an exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParagraphEditQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable base snapshot.
    pub snapshot: SnapshotId,
    /// Stable semantic paragraph identity.
    pub paragraph: EntityId,
    /// Authored replacement text.
    pub replacement: String,
}

/// Requests a shared table-cell replacement candidate at an exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentTableCellEditQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable base snapshot.
    pub snapshot: SnapshotId,
    /// Stable semantic table-cell identity.
    pub cell: EntityId,
    /// Authored replacement text.
    pub replacement: String,
}

/// Selects the amount of data returned by an inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetailLevel {
    /// Return orientation metadata and stable follow-up handles.
    Compact,
    /// Include the additional summary fields available from the application.
    Detailed,
}

/// Returns snapshot-bound orientation metadata for an entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectResult {
    /// Entity requested by the caller.
    pub entity: EntityId,
    /// Exact revision resolved by the application, when available.
    pub revision: Option<RevisionId>,
    /// Exact snapshot used by the operation, when available.
    pub snapshot: Option<SnapshotId>,
    /// Semantic type exposed by the graph/schema layer.
    pub object_type: Option<ObjectType>,
    /// Optional short stable label.
    pub label: Option<String>,
    /// Optional short description.
    pub description: Option<String>,
    /// Stable handles for deeper inspection or traversal.
    pub capabilities: Vec<String>,
    /// Trust and provenance summary for this entity.
    pub trust: Option<TrustSummary>,
}

/// Provides shared application-level read operations.
pub trait QueryService {
    /// Executes a bounded domain geospatial query through the shared query boundary.
    ///
    /// # Parameters
    /// - `self`: Application query service.
    /// - `_context`: Caller grants and request context.
    /// - `_request`: Exact project, snapshot, layer, predicate, and page bounds.
    ///
    /// # Returns
    /// Matching semantic feature summaries and exact source provenance.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default, or authorization, query, and source errors.
    fn query_geospatial_features(
        &self,
        _context: &ApplicationContext,
        _request: &crate::GeoFeatureQuery,
    ) -> Result<crate::GeoFeatureQueryResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "geospatial feature query".to_owned(),
        })
    }

    /// Reads one QGIS project's exact-snapshot layer references through shared Inspect scope.
    ///
    /// # Parameters
    /// - `self`: Shared application query service.
    /// - `_context`: Caller grants and request context.
    /// - `_request`: Exact project, snapshot, and QGIS source path.
    ///
    /// # Returns
    /// QGIS project semantics with resolved linked-layer identities.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default or authorization/read failures.
    fn inspect_qgis_project(
        &self,
        _context: &ApplicationContext,
        _request: &crate::QgisProjectQuery,
    ) -> Result<crate::QgisProjectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "qgis.project.inspect".into(),
        })
    }
    /// Reads a bounded page of persisted project artifact summaries.
    ///
    /// # Parameters
    /// - `self`: Application query service.
    /// - `_context`: Caller, capability, and snapshot context.
    /// - `_project`: Stable project identity.
    /// - `_offset`: Zero-based artifact offset.
    /// - `_limit`: Maximum number of artifact summaries.
    ///
    /// # Returns
    /// The bounded artifact page when the service supports project manifests.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default, or a structured
    /// authorization, request, or catalog error from an implementation.
    fn inspect_artifacts(
        &self,
        _context: &ApplicationContext,
        _project: ProjectId,
        _offset: usize,
        _limit: usize,
    ) -> Result<ArtifactManifestPage, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "artifact inspection".to_owned(),
        })
    }

    /// Reads one bounded page of physical and container-internal artifact members.
    ///
    /// # Parameters
    /// - `self`: Application query service.
    /// - `_context`: Caller, capability, and snapshot context.
    /// - `_project`: Stable project identity.
    /// - `_artifact`: Stable semantic artifact identity.
    /// - `_offset`: Zero-based member offset.
    /// - `_limit`: Maximum number of member descriptors.
    ///
    /// # Returns
    /// The bounded exact member page when supported.
    ///
    /// # Errors
    /// Returns unsupported-operation by default or structured authorization and catalog errors.
    fn inspect_artifact_members(
        &self,
        _context: &ApplicationContext,
        _project: ProjectId,
        _artifact: EntityId,
        _offset: usize,
        _limit: usize,
    ) -> Result<crate::ArtifactMembers, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "artifact member inspection".to_owned(),
        })
    }

    /// Reads a bounded exact-snapshot page of semantic spreadsheet cells.
    ///
    /// # Parameters
    /// - `self`: Shared application query service.
    /// - `_context`: Caller grants for the operation.
    /// - `_request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// Snapshot-bound typed spreadsheet cells with continuation metadata.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default or the structured
    /// authorization, request, storage, or decoding failure from an implementation.
    fn read_spreadsheet_cells(
        &self,
        _context: &ApplicationContext,
        _request: &SpreadsheetCellsQuery,
    ) -> Result<crate::SpreadsheetCells, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "spreadsheet cell inspection".to_owned(),
        })
    }

    /// Inspects one exact worksheet coordinate, including absent sparse cells.
    ///
    /// # Parameters
    /// - `self`: Shared application query service.
    /// - `_context`: Caller grants for the operation.
    /// - `_request`: Exact project, snapshot, worksheet, and coordinate.
    ///
    /// # Returns
    /// Typed cell data, or `None` when the coordinate is a sparse blank.
    ///
    /// # Errors
    /// Returns unsupported-operation, authorization, request, storage, or decoding failures.
    fn inspect_spreadsheet_cell(
        &self,
        _context: &ApplicationContext,
        _request: &SpreadsheetCellQuery,
    ) -> Result<crate::SpreadsheetCellInspection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "spreadsheet cell inspection".to_owned(),
        })
    }

    /// Inspects one entity under the supplied request context.
    ///
    /// - `self`: Application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Snapshot-bound inspection metadata.
    ///
    /// Returns a structured application error when inspection is unavailable or denied.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError>;

    /// Inspects one entity while observing caller cancellation.
    ///
    /// - `self`: Application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// Snapshot-bound inspection metadata when the read completes.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns `OperationCancelled` when cancellation is observed, or a
    /// structured application failure when inspection fails.
    fn inspect_cancellable(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<InspectResult, ApplicationError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "inspect".to_owned(),
            });
        }
        let result = self.inspect(context, request)?;
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "inspect".to_owned(),
            });
        }
        Ok(result)
    }
}

#[path = "query/borrowed.rs"]
mod borrowed;

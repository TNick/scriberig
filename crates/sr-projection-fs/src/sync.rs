//! Coordinates filesystem observation, import scheduling, and resynchronization.

use crate::{FailureStage, FileDigest, ProjectionLocation, ProjectionOwnership};
use serde::{Deserialize, Serialize};
use sr_projection::{
    DiagnosticDetail, DiagnosticKind, DiagnosticStage, ProjectionDiagnostic, ProjectionRef,
    ProjectionStatus, ReconciliationBaseline, SemanticStateRef,
};

mod persistence;

/// Runtime synchronization metadata for one filesystem projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SyncState {
    /// Projection instance.
    pub projection: ProjectionRef,
    /// Exact synchronized render inputs, output bytes, and fidelity evidence.
    pub baseline: ReconciliationBaseline,
    /// Physical filesystem location.
    pub location: ProjectionLocation,
    /// Ownership policy for this location.
    pub ownership: ProjectionOwnership,
    /// Current canonical semantic state when it differs from the base.
    pub current: SemanticStateRef,
    /// Last observed exact-byte digest, when an observation occurred.
    pub observed: Option<FileDigest>,
    /// Exact digest of a committed agent write awaiting watcher observation.
    pub self_write_digest: Option<FileDigest>,
    /// Current synchronization lifecycle status.
    pub status: ProjectionStatus,
}

impl SyncState {
    /// Creates a synchronization record from one exact rendered baseline.
    ///
    /// - `projection`: Stable projection instance.
    /// - `baseline`: Exact output and immutable inputs that produced it.
    /// - `location`: Physical filesystem location.
    /// - `ownership`: Write/import policy for the location.
    /// - `current`: Current canonical semantic state.
    /// - `status`: Initial synchronization lifecycle status.
    ///
    /// # Returns
    /// Synchronization record with no pending observation or self-write suppression.
    #[must_use]
    pub fn new(
        projection: ProjectionRef,
        baseline: ReconciliationBaseline,
        location: ProjectionLocation,
        ownership: ProjectionOwnership,
        current: SemanticStateRef,
        status: ProjectionStatus,
    ) -> Self {
        Self {
            projection,
            baseline,
            location,
            ownership,
            current,
            observed: None,
            self_write_digest: None,
            status,
        }
    }
}

/// Exact three-way input scheduled for domain parsing and reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportRequest {
    /// Projection being imported.
    pub projection: ProjectionRef,
    /// Exact synchronized output and the inputs that produced it.
    pub baseline: ReconciliationBaseline,
    /// Current canonical semantic state.
    pub current: SemanticStateRef,
    /// Exact external file bytes.
    pub bytes: Vec<u8>,
    /// Exact external-byte digest.
    pub digest: FileDigest,
    /// External location at observation time.
    pub location: ProjectionLocation,
}

/// Result returned by filesystem observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationResult {
    /// State after applying the observation hint.
    pub status: ProjectionStatus,
    /// Domain import work when semantic import is permitted.
    pub import: Option<ImportRequest>,
}

/// Completion supplied by the domain import pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportCompletion {
    /// External bytes were semantically unchanged and need canonical rerendering.
    NoSemanticChange {
        /// Original import request.
        request: ImportRequest,
        /// New canonical bytes to write.
        bytes: Vec<u8>,
        /// Semantic state that is now synchronized.
        base: SemanticStateRef,
        /// Exact new output baseline established after successful synchronization.
        baseline: ReconciliationBaseline,
    },
    /// A validated migration was committed and rendered.
    Committed {
        /// Original import request.
        request: ImportRequest,
        /// New canonical bytes to write.
        bytes: Vec<u8>,
        /// Semantic state produced by the committed migration.
        base: SemanticStateRef,
        /// Exact new output baseline established after successful synchronization.
        baseline: ReconciliationBaseline,
    },
    /// Import failed while retaining retryable external evidence.
    Failed {
        /// Original import request.
        request: ImportRequest,
        /// Failure stage and structured diagnostic.
        stage: FailureStage,
        /// Diagnostic explaining the failure.
        diagnostic: ProjectionDiagnostic,
    },
    /// Import reached an unresolved conflict.
    Conflict {
        /// Original import request.
        request: ImportRequest,
        /// Diagnostic explaining the conflict.
        diagnostic: ProjectionDiagnostic,
    },
}

impl ImportCompletion {
    /// Creates a retryable parse failure for an import request.
    ///
    /// - `request`: Exact three-way import inputs.
    /// - `message`: Human-readable parse failure.
    ///
    /// A structured failed-import completion.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn failed(request: ImportRequest, message: impl Into<String>) -> Self {
        let diagnostic = ProjectionDiagnostic {
            kind: DiagnosticKind::ParseFailure,
            stage: DiagnosticStage::Parse,
            projection: Some(request.projection),
            scope: Some(request.baseline.rendered.cache_key.scope),
            detail: DiagnosticDetail {
                code: "import-failed".to_owned(),
                data: Vec::new(),
            },
            source: None,
            message: Some(message.into()),
            retryable: true,
        };
        Self::Failed {
            request,
            stage: FailureStage::Parse,
            diagnostic,
        }
    }
}

/// In-memory synchronization state boundary suitable for deterministic tests.
#[derive(Clone, Debug, Default)]
pub struct MemorySyncStore {
    states: Vec<SyncState>,
    persistence_path: Option<std::path::PathBuf>,
}

impl MemorySyncStore {
    /// Creates or replaces a projection synchronization record.
    ///
    /// - `state`: State record to store.
    pub fn put(&mut self, state: SyncState) {
        if let Some(existing) = self
            .states
            .iter_mut()
            .find(|existing| existing.projection == state.projection)
        {
            *existing = state;
        } else {
            self.states.push(state);
        }
    }

    /// Finds state by physical location.
    ///
    /// - `location`: Location to find.
    ///
    /// Matching state when present.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn find(&self, location: &ProjectionLocation) -> Option<&SyncState> {
        self.states.iter().find(|state| &state.location == location)
    }

    /// Finds mutable state by physical location.
    ///
    /// - `location`: Location to find.
    ///
    /// Mutable matching state when present.
    /// # Returns
    /// The operation result.
    pub fn find_mut(&mut self, location: &ProjectionLocation) -> Option<&mut SyncState> {
        self.states
            .iter_mut()
            .find(|state| &state.location == location)
    }

    /// Finds state by projection identity.
    ///
    /// - `projection`: Projection to find.
    ///
    /// Matching state when present.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn find_projection(&self, projection: ProjectionRef) -> Option<&SyncState> {
        self.states
            .iter()
            .find(|state| state.projection == projection)
    }

    /// Finds mutable state by projection identity.
    ///
    /// - `projection`: Projection to find.
    ///
    /// Mutable matching state when present.
    /// # Returns
    /// The operation result.
    pub fn find_projection_mut(&mut self, projection: ProjectionRef) -> Option<&mut SyncState> {
        self.states
            .iter_mut()
            .find(|state| state.projection == projection)
    }

    /// Moves a synchronization record after a rename observation.
    ///
    /// - `from`: Previous physical location.
    /// - `to`: New physical location.
    ///
    /// `true` when a record was moved.
    /// # Returns
    /// The operation result.
    pub fn rename(&mut self, from: &ProjectionLocation, to: ProjectionLocation) -> bool {
        let Some(state) = self.states.iter_mut().find(|state| &state.location == from) else {
            return false;
        };
        state.location = to;
        true
    }
}

pub use crate::coordinator::inspect_observation;

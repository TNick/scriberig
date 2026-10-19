//! Coordinates filesystem observation, import scheduling, and resynchronization.

use crate::{FailureStage, FileDigest, ProjectionLocation, ProjectionOwnership};
use sr_projection::{
    DiagnosticDetail, DiagnosticKind, DiagnosticStage, ProjectionDiagnostic, ProjectionKind,
    ProjectionRef, ProjectionSpecRef, ProjectionStatus, RendererVersion, SemanticScopeRef,
    SemanticStateRef, StyleRef,
};

/// Runtime synchronization metadata for one filesystem projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncState {
    /// Projection instance.
    pub projection: ProjectionRef,
    /// Persistent projection specification reference.
    pub specification: ProjectionSpecRef,
    /// Semantic scope represented by the projection.
    pub scope: SemanticScopeRef,
    /// Adapter-defined representation kind used for parsing and rendering.
    pub kind: ProjectionKind,
    /// Physical filesystem location.
    pub location: ProjectionLocation,
    /// Ownership policy for this location.
    pub ownership: ProjectionOwnership,
    /// Last successfully synchronized semantic base.
    pub base: SemanticStateRef,
    /// Current canonical semantic state when it differs from the base.
    pub current: SemanticStateRef,
    /// Exact style input used for the synchronized render.
    pub style: StyleRef,
    /// Exact renderer version used for the synchronized render.
    pub renderer: RendererVersion,
    /// Last exact rendered-byte digest.
    pub rendered: FileDigest,
    /// Last observed exact-byte digest, when an observation occurred.
    pub observed: Option<FileDigest>,
    /// Current synchronization lifecycle status.
    pub status: ProjectionStatus,
}

/// Exact three-way input scheduled for domain parsing and reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportRequest {
    /// Projection being imported.
    pub projection: ProjectionRef,
    /// Persistent specification used by the projection.
    pub specification: ProjectionSpecRef,
    /// Semantic scope being reconciled.
    pub scope: SemanticScopeRef,
    /// Adapter-defined representation kind used for parsing.
    pub kind: ProjectionKind,
    /// Last synchronized semantic state.
    pub base: SemanticStateRef,
    /// Current canonical semantic state.
    pub current: SemanticStateRef,
    /// Exact external file bytes.
    pub bytes: Vec<u8>,
    /// Exact external-byte digest.
    pub digest: FileDigest,
    /// External location at observation time.
    pub location: ProjectionLocation,
    /// Style used by the synchronized base.
    pub style: StyleRef,
    /// Renderer version used by the synchronized base.
    pub renderer: RendererVersion,
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
    },
    /// A validated migration was committed and rendered.
    Committed {
        /// Original import request.
        request: ImportRequest,
        /// New canonical bytes to write.
        bytes: Vec<u8>,
        /// Semantic state produced by the committed migration.
        base: SemanticStateRef,
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
            scope: Some(request.scope),
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

//! Runs adapter parsing, reconciliation, promotion, and rerendering.

use crate::coordinator::SyncCoordinator;
use crate::{FailureStage, FileSystem, ImportCompletion, ImportRequest, IoError};
use sr_projection::{
    ProjectionAdapter, ProjectionDiagnostic, ProjectionRef, ReconciliationContext,
    ReconciliationOutcome, RenderRequest, RenderingContext, SemanticStateRef,
};

/// Error returned when adapter rendering or filesystem synchronization fails.
#[derive(Debug)]
pub enum RenderError {
    /// The filesystem boundary failed.
    Io(IoError),
    /// The adapter rejected the render request.
    Diagnostic(Box<ProjectionDiagnostic>),
}

impl std::fmt::Display for RenderError {
    /// Formats a render failure.
    ///
    /// - `self`: Render failure.
    /// - `formatter`: Formatting destination.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when the destination rejects the message.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "filesystem render failed: {error}"),
            Self::Diagnostic(diagnostic) => {
                write!(formatter, "adapter render failed: {diagnostic:?}")
            }
        }
    }
}

impl std::error::Error for RenderError {}

impl<F: FileSystem> SyncCoordinator<F> {
    /// Renders a managed projection through its adapter and writes it atomically.
    ///
    /// - `projection`: Projection to render.
    /// - `adapter`: Domain adapter that owns deterministic rendering.
    ///
    /// Unit after the file and synchronization state are updated.
    ///
    /// Returns an adapter diagnostic or filesystem failure when rendering cannot complete.
    pub fn render_managed_projection<A: ProjectionAdapter>(
        &mut self,
        projection: ProjectionRef,
        adapter: &A,
    ) -> Result<(), RenderError> {
        let state = self
            .states()
            .find_projection(projection)
            .cloned()
            .ok_or(RenderError::Io(IoError::OwnershipDenied))?;
        let request = RenderRequest {
            specification: state.specification,
            scope: state.scope,
            base_state: state.current,
            kind: state.kind,
            style: state.style,
            renderer: state.renderer,
            context: RenderingContext::default(),
        };
        let rendered = adapter.render(&request).map_err(RenderError::Diagnostic)?;
        self.write_rendered(projection, &rendered.bytes, state.current)
            .map_err(RenderError::Io)
    }

    /// Runs the adapter import lifecycle and synchronizes an accepted result.
    ///
    /// - `request`: Exact import request returned by [`Self::observe`].
    /// - `adapter`: Domain adapter that owns parsing and reconciliation.
    /// - `promote`: Callback that validates and promotes a proposed migration.
    /// - `conflict`: Callback that converts an adapter conflict into a diagnostic.
    ///
    /// Unit after the projection is synchronized or retryable evidence is retained.
    ///
    /// Returns an IO or ownership error when synchronization state cannot be updated.
    pub fn process_import<A, Promote, Conflict>(
        &mut self,
        request: ImportRequest,
        adapter: &A,
        mut promote: Promote,
        conflict: Conflict,
    ) -> Result<(), IoError>
    where
        A: ProjectionAdapter,
        Promote: FnMut(
            A::Migration,
            &ImportRequest,
        ) -> Result<SemanticStateRef, Box<ProjectionDiagnostic>>,
        Conflict: FnOnce(A::Conflict) -> ProjectionDiagnostic,
    {
        let candidate = match adapter.parse(&request.kind, &request.bytes) {
            Ok(candidate) => candidate,
            Err(diagnostic) => {
                return self.complete_import(
                    request.projection,
                    ImportCompletion::Failed {
                        request,
                        stage: FailureStage::Parse,
                        diagnostic: *diagnostic,
                    },
                );
            }
        };
        if let Err(diagnostic) = adapter.validate(&candidate) {
            return self.complete_import(
                request.projection,
                ImportCompletion::Failed {
                    request,
                    stage: FailureStage::Validate,
                    diagnostic: *diagnostic,
                },
            );
        }
        let context = ReconciliationContext {
            base: request.base,
            current: request.current,
            scope: request.scope,
            specification: request.specification,
            metadata: std::collections::BTreeMap::new(),
        };
        match adapter.reconcile(candidate, &context) {
            ReconciliationOutcome::NoSemanticChange => {
                let base = request.current;
                self.rerender_import(request, adapter, base)
            }
            ReconciliationOutcome::MigrationProposal(migration) => {
                let base = match promote(migration, &request) {
                    Ok(base) => base,
                    Err(diagnostic) => {
                        return self.complete_import(
                            request.projection,
                            ImportCompletion::Failed {
                                request,
                                stage: FailureStage::Commit,
                                diagnostic: *diagnostic,
                            },
                        );
                    }
                };
                self.rerender_import(request, adapter, base)
            }
            ReconciliationOutcome::Ambiguous(diagnostic) => self.complete_import(
                request.projection,
                ImportCompletion::Conflict {
                    request,
                    diagnostic: *diagnostic,
                },
            ),
            ReconciliationOutcome::InvalidExternalState(diagnostic) => self.complete_import(
                request.projection,
                ImportCompletion::Failed {
                    request,
                    stage: FailureStage::Reconcile,
                    diagnostic: *diagnostic,
                },
            ),
            ReconciliationOutcome::Conflict(conflict_value) => self.complete_import(
                request.projection,
                ImportCompletion::Conflict {
                    request,
                    diagnostic: conflict(conflict_value),
                },
            ),
        }
    }

    /// Renders an adapter-approved semantic state and writes it atomically.
    ///
    /// - `request`: Original external import request.
    /// - `adapter`: Domain adapter that owns rendering.
    /// - `base`: Semantic state represented by the rendered bytes.
    ///
    /// Unit after the accepted state is synchronized.
    ///
    /// Returns an IO or ownership error when state cannot be synchronized.
    fn rerender_import<A: ProjectionAdapter>(
        &mut self,
        request: ImportRequest,
        adapter: &A,
        base: SemanticStateRef,
    ) -> Result<(), IoError> {
        let render_request = RenderRequest {
            specification: request.specification,
            scope: request.scope,
            base_state: base,
            kind: request.kind.clone(),
            style: request.style.clone(),
            renderer: request.renderer.clone(),
            context: RenderingContext::default(),
        };
        let rendered = match adapter.render(&render_request) {
            Ok(rendered) => rendered,
            Err(diagnostic) => {
                return self.complete_import(
                    request.projection,
                    ImportCompletion::Failed {
                        request,
                        stage: FailureStage::Rerender,
                        diagnostic: *diagnostic,
                    },
                );
            }
        };
        self.complete_import(
            request.projection,
            ImportCompletion::NoSemanticChange {
                request,
                bytes: rendered.bytes,
                base,
            },
        )
    }
}

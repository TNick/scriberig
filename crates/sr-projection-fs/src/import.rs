//! Runs adapter parsing, reconciliation, promotion, and rerendering.

use crate::coordinator::SyncCoordinator;
use crate::{FailureStage, FileSystem, ImportCompletion, ImportRequest, IoError};
use sr_projection::{
    ProjectionAdapter, ProjectionDiagnostic, ProjectionRef, ReconciliationBaseline,
    ReconciliationContext, ReconciliationOutcome, RenderRequest, RenderingContext, ResolvedStyle,
    SemanticStateRef, invalid_style_diagnostic,
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
    /// Renders a managed projection using the style recorded in its state.
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
        let style = state.baseline.rendered.style.clone();
        self.render_managed_projection_with_style(projection, adapter, &style)
    }

    /// Renders a managed projection under one exact style supplied by the caller.
    ///
    /// Materialization is a shared purpose: the caller resolves the style once
    /// (workspace materialization policy, never a personal preference or view
    /// override) and hands the exact value down. The filesystem layer therefore
    /// never consults configuration state, and personal style cannot reach a
    /// materialized file through this path.
    ///
    /// - `projection`: Projection to render.
    /// - `adapter`: Domain adapter that owns deterministic rendering.
    /// - `style`: Exact resolved style the rendered bytes must carry.
    ///
    /// Unit after the file and synchronization state are updated. The recorded
    /// style becomes the one used by later observation and import reconciliation,
    /// so a style change cannot be reconciled against stale render identity.
    ///
    /// # Errors
    /// Returns an adapter diagnostic or filesystem failure when rendering cannot
    /// complete.
    pub fn render_managed_projection_with_style<A: ProjectionAdapter>(
        &mut self,
        projection: ProjectionRef,
        adapter: &A,
        style: &ResolvedStyle,
    ) -> Result<(), RenderError> {
        let state = self
            .states()
            .find_projection(projection)
            .cloned()
            .ok_or(RenderError::Io(IoError::OwnershipDenied))?;
        let request = RenderRequest {
            specification: state.baseline.rendered.cache_key.specification,
            scope: state.baseline.rendered.cache_key.scope,
            base_state: state.current,
            kind: state.baseline.rendered.kind.clone(),
            style: style.clone(),
            renderer: state.baseline.rendered.renderer.clone(),
            context: RenderingContext::default(),
        };
        let rendered = adapter.render(&request).map_err(RenderError::Diagnostic)?;
        let baseline = ReconciliationBaseline::new(
            rendered.clone(),
            state.baseline.adapter.clone(),
            state.baseline.fidelity.clone(),
        )
        .map_err(|_| RenderError::Io(crate::IoError::BaselineMismatch))?;
        self.write_rendered(projection, &rendered.bytes, state.current)
            .map_err(RenderError::Io)?;
        let Some(state) = self.states_mut().find_projection_mut(projection) else {
            return Err(RenderError::Io(crate::IoError::OwnershipDenied));
        };
        state.baseline = baseline;
        self.states().persist().map_err(RenderError::Io)?;
        Ok(())
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
        let kind = &request.baseline.rendered.kind;
        if !request.baseline.fidelity.fidelity.is_editable() {
            let diagnostic = invalid_style_diagnostic(
                kind,
                "projection-baseline-read-only",
                "this exact projection baseline is not editable",
                Vec::new(),
            );
            return self.complete_import(
                request.projection,
                ImportCompletion::Failed {
                    request,
                    stage: FailureStage::Validate,
                    diagnostic,
                },
            );
        }
        let candidate = match adapter.parse(kind, &request.bytes) {
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
            baseline: request.baseline.clone(),
            current: request.current,
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
            specification: request.baseline.rendered.cache_key.specification,
            scope: request.baseline.rendered.cache_key.scope,
            base_state: base,
            kind: request.baseline.rendered.kind.clone(),
            style: request.baseline.rendered.style.clone(),
            renderer: request.baseline.rendered.renderer.clone(),
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
        let baseline = match ReconciliationBaseline::new(
            rendered.clone(),
            request.baseline.adapter.clone(),
            request.baseline.fidelity.clone(),
        ) {
            Ok(baseline) => baseline,
            Err(error) => {
                let diagnostic = invalid_style_diagnostic(
                    &request.baseline.rendered.kind,
                    "invalid-rendered-baseline",
                    error,
                    Vec::new(),
                );
                return self.complete_import(
                    request.projection,
                    ImportCompletion::Failed {
                        request,
                        stage: FailureStage::Rerender,
                        diagnostic,
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
                baseline,
            },
        )
    }
}

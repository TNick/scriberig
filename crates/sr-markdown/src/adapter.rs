//! Implements the generic projection adapter for Markdown documents.

use sr_projection::{
    DiagnosticKind, DiagnosticStage, ProjectionAdapter, ProjectionCapabilities,
    ProjectionDiagnostic, ProjectionKind, ProjectionResult, ReconciliationContext,
    ReconciliationOutcome, RenderRequest, RenderedProjection,
};

use crate::{
    CandidateDocument, CanonicalNode, MarkdownConflict, MarkdownDocument,
    MarkdownMigrationProposal, MarkdownReconciliation, MarkdownStyle, ReconciliationAmbiguity,
    ThreeWayOutcome, parse, reconcile, render_document, three_way_merge,
};

/// Stable projection kind identifier for Markdown.
pub const MARKDOWN_KIND: &str = "markdown";

/// Adapter that parses and renders one canonical Markdown semantic document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownAdapter {
    /// Canonical document used by rendering and local reconciliation.
    pub document: MarkdownDocument,
    /// Exact deterministic formatting style.
    pub style: MarkdownStyle,
    /// Canonical projected nodes used to preserve identity during import.
    pub canonical_nodes: Vec<CanonicalNode>,
    /// Last synchronized semantic document used as the three-way base.
    pub synchronization_base: Option<MarkdownDocument>,
}

impl MarkdownAdapter {
    /// Creates an adapter for a canonical document and validates its style.
    ///
    /// # Errors
    /// Returns the style validation message when the style is unsupported.
    ///
    /// # Parameters
    /// - `document`: Canonical Markdown document.
    /// - `style`: Exact rendering style.
    ///
    /// # Returns
    /// A configured Markdown adapter.
    pub fn new(document: MarkdownDocument, style: MarkdownStyle) -> Result<Self, String> {
        Ok(Self {
            document,
            style: style.validate()?,
            canonical_nodes: Vec::new(),
            synchronization_base: None,
        })
    }

    /// Creates an adapter using the deterministic default style.
    ///
    /// # Parameters
    /// - `document`: Canonical Markdown document.
    ///
    /// # Returns
    /// A configured adapter using [`MarkdownStyle::default`].
    #[must_use]
    pub fn with_default_style(document: MarkdownDocument) -> Self {
        Self {
            document,
            style: MarkdownStyle::default(),
            canonical_nodes: Vec::new(),
            synchronization_base: None,
        }
    }

    /// Creates an adapter with the canonical nodes for its projected scope.
    ///
    /// # Parameters
    /// - `document`: Canonical Markdown document used for rendering.
    /// - `canonical_nodes`: Stable identities for the projected semantic nodes.
    /// - `style`: Exact rendering style.
    ///
    /// # Returns
    /// A configured editable Markdown adapter.
    ///
    /// # Errors
    /// Returns the style validation message when the style is unsupported.
    pub fn with_canonical_nodes(
        document: MarkdownDocument,
        canonical_nodes: Vec<CanonicalNode>,
        style: MarkdownStyle,
    ) -> Result<Self, String> {
        Ok(Self {
            document,
            canonical_nodes,
            style: style.validate()?,
            synchronization_base: None,
        })
    }

    /// Creates an editable adapter with an explicit three-way base document.
    ///
    /// # Parameters
    /// - `document`: Current canonical Markdown document.
    /// - `canonical_nodes`: Stable identities from the synchronization base.
    /// - `synchronization_base`: Markdown document last synchronized to the file.
    /// - `style`: Exact rendering style.
    ///
    /// # Returns
    /// A configured editable Markdown adapter.
    ///
    /// # Errors
    /// Returns the style validation message when the style is unsupported.
    pub fn with_canonical_nodes_and_base(
        document: MarkdownDocument,
        canonical_nodes: Vec<CanonicalNode>,
        synchronization_base: MarkdownDocument,
        style: MarkdownStyle,
    ) -> Result<Self, String> {
        Ok(Self {
            document,
            canonical_nodes,
            synchronization_base: Some(synchronization_base),
            style: style.validate()?,
        })
    }
}

impl ProjectionAdapter for MarkdownAdapter {
    type Candidate = CandidateDocument;
    type Migration = Box<MarkdownMigrationProposal>;
    type Conflict = Vec<MarkdownConflict>;

    /// Returns capabilities for one projection kind.
    ///
    /// # Parameters
    /// - `kind`: Projection format to inspect.
    ///
    /// # Returns
    /// Capabilities supported for the requested format.
    fn capabilities(&self, kind: &ProjectionKind) -> ProjectionCapabilities {
        if kind.as_str() == MARKDOWN_KIND {
            ProjectionCapabilities::editable()
        } else {
            ProjectionCapabilities::read_only()
        }
    }

    /// Renders the configured document using the generic request metadata.
    ///
    /// # Parameters
    /// - `request`: Exact generic projection render inputs.
    ///
    /// # Returns
    /// Rendered Markdown bytes and output metadata.
    ///
    /// # Errors
    /// Returns a diagnostic for an unsupported projection kind.
    fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
        if request.kind.as_str() != MARKDOWN_KIND {
            return Err(Box::new(diagnostic(
                DiagnosticStage::Render,
                DiagnosticKind::UnsupportedOperation,
                "unsupported-kind",
                "Markdown adapter received a non-Markdown projection kind",
            )));
        }
        let bytes = render_document(&self.document, &self.style).into_bytes();
        Ok(RenderedProjection::from_bytes(bytes, request, None))
    }

    /// Parses external Markdown bytes into a candidate document.
    ///
    /// # Parameters
    /// - `kind`: Projection format being parsed.
    /// - `bytes`: Exact external representation bytes.
    ///
    /// # Returns
    /// An adapter-owned Markdown candidate.
    ///
    /// # Errors
    /// Returns a diagnostic for invalid UTF-8, unsupported format, or invalid Markdown.
    fn parse(&self, kind: &ProjectionKind, bytes: &[u8]) -> ProjectionResult<Self::Candidate> {
        if kind.as_str() != MARKDOWN_KIND {
            return Err(Box::new(diagnostic(
                DiagnosticStage::Parse,
                DiagnosticKind::UnsupportedOperation,
                "unsupported-kind",
                "Markdown adapter received a non-Markdown projection kind",
            )));
        }
        let source = std::str::from_utf8(bytes).map_err(|error| {
            Box::new(diagnostic(
                DiagnosticStage::Parse,
                DiagnosticKind::ParseFailure,
                "invalid-utf8",
                error.to_string(),
            ))
        })?;
        parse(source).map_err(|error| {
            Box::new(diagnostic(
                DiagnosticStage::Parse,
                DiagnosticKind::ParseFailure,
                &error.code,
                &error.message,
            ))
        })
    }

    /// Compares a candidate with this adapter's canonical document.
    ///
    /// # Parameters
    /// - `candidate`: Parsed external candidate.
    /// - `context`: Exact canonical and projection context supplied by the runtime.
    ///
    /// # Returns
    /// A no-op outcome for equal semantics or a Markdown migration proposal.
    fn reconcile(
        &self,
        candidate: Self::Candidate,
        context: &ReconciliationContext,
    ) -> ReconciliationOutcome<Self::Migration, Self::Conflict, Box<ProjectionDiagnostic>> {
        if candidate.document == self.document {
            return ReconciliationOutcome::NoSemanticChange;
        }
        if self.canonical_nodes.is_empty() {
            return ReconciliationOutcome::InvalidExternalState(Box::new(diagnostic(
                DiagnosticStage::Reconcile,
                DiagnosticKind::ValidationFailure,
                "missing-canonical-identities",
                "editable Markdown reconciliation requires canonical node identities",
            )));
        }
        if context.current != context.base {
            let Some(base_document) = self.synchronization_base.as_ref() else {
                return ReconciliationOutcome::InvalidExternalState(Box::new(diagnostic(
                    DiagnosticStage::Reconcile,
                    DiagnosticKind::ReconciliationConflict,
                    "missing-synchronization-base-document",
                    "concurrent Markdown reconciliation requires the last synchronized document",
                )));
            };
            return match three_way_merge(Some(base_document), &self.document, &candidate.document) {
                ThreeWayOutcome::Conflict { conflicts } => {
                    ReconciliationOutcome::Conflict(conflicts)
                }
                ThreeWayOutcome::MissingSynchronizationBase => {
                    ReconciliationOutcome::InvalidExternalState(Box::new(diagnostic(
                        DiagnosticStage::Reconcile,
                        DiagnosticKind::ReconciliationConflict,
                        "missing-synchronization-base-document",
                        "Markdown reconciliation has no usable synchronization base",
                    )))
                }
                ThreeWayOutcome::FormattingOnly { document }
                | ThreeWayOutcome::Merged { document }
                | ThreeWayOutcome::Converged { document } => {
                    let merged = CandidateDocument::new(document, candidate.source_length);
                    match reconcile(
                        context.base.0,
                        &self.canonical_nodes,
                        &merged,
                        MARKDOWN_KIND,
                    ) {
                        MarkdownReconciliation::NoSemanticChange => {
                            ReconciliationOutcome::NoSemanticChange
                        }
                        MarkdownReconciliation::MigrationProposal(proposal) => {
                            ReconciliationOutcome::MigrationProposal(proposal)
                        }
                        MarkdownReconciliation::Ambiguous(ambiguities) => {
                            ReconciliationOutcome::Ambiguous(Box::new(ambiguity_diagnostic(
                                &ambiguities,
                            )))
                        }
                    }
                }
            };
        }
        match reconcile(
            context.base.0,
            &self.canonical_nodes,
            &candidate,
            MARKDOWN_KIND,
        ) {
            MarkdownReconciliation::NoSemanticChange => ReconciliationOutcome::NoSemanticChange,
            MarkdownReconciliation::MigrationProposal(proposal) => {
                ReconciliationOutcome::MigrationProposal(proposal)
            }
            MarkdownReconciliation::Ambiguous(ambiguities) => {
                let detail = ambiguities
                    .iter()
                    .map(|ambiguity| {
                        format!(
                            "path={:?};possible={:?}",
                            ambiguity.candidate_path, ambiguity.possible
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut diagnostic = diagnostic(
                    DiagnosticStage::Reconcile,
                    DiagnosticKind::ReconciliationAmbiguous,
                    "ambiguous-node-matching",
                    format!("Markdown node matching is ambiguous at {detail}"),
                );
                diagnostic.detail.data = detail.into_bytes();
                ReconciliationOutcome::Ambiguous(Box::new(diagnostic))
            }
        }
    }

    /// Validates candidate source metadata and semantic structure.
    ///
    /// # Parameters
    /// - `candidate`: Candidate to validate.
    ///
    /// # Returns
    /// Success when candidate metadata is consistent.
    ///
    /// # Errors
    /// Returns a diagnostic when source metadata is inconsistent.
    fn validate(&self, candidate: &Self::Candidate) -> ProjectionResult<()> {
        if candidate.source_length == 0 && !candidate.document.blocks.is_empty() {
            return Err(Box::new(diagnostic(
                DiagnosticStage::Validate,
                DiagnosticKind::ValidationFailure,
                "candidate-source-length",
                "candidate source metadata is inconsistent",
            )));
        }
        let rendered = render_document(&candidate.document, &self.style);
        let reparsed = parse(&rendered).map_err(|error| {
            Box::new(diagnostic(
                DiagnosticStage::Validate,
                DiagnosticKind::ValidationFailure,
                &error.code,
                &error.message,
            ))
        })?;
        if reparsed.document != candidate.document {
            return Err(Box::new(diagnostic(
                DiagnosticStage::Validate,
                DiagnosticKind::ValidationFailure,
                "render-parse-mismatch",
                "rendered Markdown does not preserve candidate semantics",
            )));
        }
        Ok(())
    }
}

/// Creates a generic projection diagnostic for an adapter-local failure.
///
/// # Parameters
/// - `stage`: Lifecycle stage that produced the diagnostic.
/// - `kind`: Typed diagnostic category.
/// - `code`: Stable diagnostic code.
/// - `message`: Human-readable explanation.
///
/// # Returns
/// A parse-stage projection diagnostic.
fn diagnostic(
    stage: DiagnosticStage,
    kind: DiagnosticKind,
    code: &str,
    message: impl Into<String>,
) -> ProjectionDiagnostic {
    let message = message.into();
    ProjectionDiagnostic {
        kind,
        stage,
        projection: None,
        scope: None,
        detail: sr_projection::DiagnosticDetail {
            code: code.to_owned(),
            data: Vec::new(),
        },
        source: Some(sr_projection::DiagnosticSource {
            subsystem: "sr-markdown".to_owned(),
            code: code.to_owned(),
            message: Some(message.clone()),
        }),
        message: Some(message),
        retryable: false,
    }
}

/// Creates a diagnostic for ambiguous Markdown identity matching.
///
/// # Parameters
/// - `ambiguities`: Candidate paths with multiple possible identities.
///
/// # Returns
/// A structured reconciliation diagnostic.
fn ambiguity_diagnostic(ambiguities: &[ReconciliationAmbiguity]) -> ProjectionDiagnostic {
    let detail = ambiguities
        .iter()
        .map(|ambiguity| {
            format!(
                "path={:?};possible={:?}",
                ambiguity.candidate_path, ambiguity.possible
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut diagnostic = diagnostic(
        DiagnosticStage::Reconcile,
        DiagnosticKind::ReconciliationAmbiguous,
        "ambiguous-node-matching",
        format!("Markdown node matching is ambiguous at {detail}"),
    );
    diagnostic.detail.data = detail.into_bytes();
    diagnostic
}

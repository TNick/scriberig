//! Implements the generic projection adapter for Markdown documents.

use sr_projection::{
    ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic, ProjectionKind,
    ProjectionResult, ReconciliationOutcome, RenderRequest, RenderedProjection, SemanticStateRef,
};

use crate::{CandidateDocument, MarkdownDocument, MarkdownStyle, parse, render_document};

/// Stable projection kind identifier for Markdown.
pub const MARKDOWN_KIND: &str = "markdown";

/// Adapter that parses and renders one canonical Markdown semantic document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownAdapter {
    /// Canonical document used by rendering and local reconciliation.
    pub document: MarkdownDocument,
    /// Exact deterministic formatting style.
    pub style: MarkdownStyle,
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
        }
    }
}

impl ProjectionAdapter for MarkdownAdapter {
    type Candidate = CandidateDocument;
    type Migration = MarkdownDocument;
    type Conflict = MarkdownDocument;

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
                "unsupported-kind",
                "Markdown adapter received a non-Markdown projection kind",
            )));
        }
        let source = std::str::from_utf8(bytes)
            .map_err(|error| Box::new(diagnostic("invalid-utf8", error.to_string())))?;
        parse(source).map_err(|error| Box::new(diagnostic(&error.code, &error.message)))
    }

    /// Compares a candidate with this adapter's canonical document.
    ///
    /// # Parameters
    /// - `candidate`: Parsed external candidate.
    /// - `_base`: Exact canonical base supplied by the projection runtime.
    ///
    /// # Returns
    /// A no-op outcome for equal semantics or a Markdown migration proposal.
    fn reconcile(
        &self,
        candidate: Self::Candidate,
        _base: SemanticStateRef,
    ) -> ReconciliationOutcome<Self::Migration, Self::Conflict, Box<ProjectionDiagnostic>> {
        if candidate.document == self.document {
            ReconciliationOutcome::NoSemanticChange
        } else {
            ReconciliationOutcome::MigrationProposal(candidate.document)
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
                "candidate-source-length",
                "candidate source metadata is inconsistent",
            )));
        }
        Ok(())
    }
}

/// Creates a generic projection diagnostic for an adapter-local failure.
///
/// # Parameters
/// - `code`: Stable diagnostic code.
/// - `message`: Human-readable explanation.
///
/// # Returns
/// A parse-stage projection diagnostic.
fn diagnostic(code: &str, message: impl Into<String>) -> ProjectionDiagnostic {
    ProjectionDiagnostic {
        stage: sr_projection::DiagnosticStage::Parse,
        projection: None,
        scope: None,
        detail: sr_projection::DiagnosticDetail {
            code: code.to_owned(),
            data: Vec::new(),
        },
        message: Some(message.into()),
        retryable: false,
    }
}

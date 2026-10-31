//! Routes one style seam to the domain adapter that owns each projection kind.

use std::sync::Arc;

use sr_projection::{
    FidelityAssessment, ProjectionDiagnostic, ProjectionKind, ProjectionPurpose,
    ProjectionStyleAdapter, ResolvedStyle, StyleSchema, StyleValue, unsupported_style_diagnostic,
};

/// A domain style adapter registered for one projection kind.
///
/// # Public members
/// - `kind`: Projection kind the adapter renders.
/// - `adapter`: Domain adapter owning style meaning for that kind.
///
/// # Private members
/// - None.
pub struct RegisteredStyleAdapter {
    /// Projection kind the adapter renders.
    pub kind: String,
    /// Domain adapter owning style meaning for that kind.
    pub adapter: Arc<dyn ProjectionStyleAdapter>,
}

/// A style adapter that dispatches by projection kind.
///
/// A host composition installs domain adapters side by side without flattening
/// their meaning: each kind keeps its own schema, defaults, normalization, merge
/// rules, and fidelity, and an unregistered kind is reported as unsupported
/// instead of receiving a generic style.
///
/// # Public members
/// - `new`: Builds an empty dispatcher.
/// - `with_adapter`: Registers one adapter for one kind.
///
/// # Private members
/// - `entries`: Registered kind/adapter pairs in registration order.
pub struct KindDispatchStyleAdapter {
    /// Registered kind/adapter pairs in registration order.
    entries: Vec<RegisteredStyleAdapter>,
}

impl Default for KindDispatchStyleAdapter {
    /// Builds an empty dispatcher.
    ///
    /// # Returns
    /// The dispatcher with no registered domains.
    fn default() -> Self {
        Self::new()
    }
}

impl KindDispatchStyleAdapter {
    /// Builds an empty dispatcher.
    ///
    /// # Returns
    /// The dispatcher with no registered domains.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registers one domain adapter for one projection kind.
    ///
    /// - `self`: Dispatcher being extended.
    /// - `kind`: Projection kind the adapter renders.
    /// - `adapter`: Domain adapter owning style meaning for that kind.
    ///
    /// # Returns
    /// The dispatcher with the adapter registered.
    #[must_use]
    pub fn with_adapter(
        mut self,
        kind: impl Into<String>,
        adapter: Arc<dyn ProjectionStyleAdapter>,
    ) -> Self {
        self.entries.push(RegisteredStyleAdapter {
            kind: kind.into(),
            adapter,
        });
        self
    }

    /// Returns the adapter registered for one kind.
    ///
    /// - `self`: Dispatcher to consult.
    /// - `kind`: Projection kind being styled.
    ///
    /// # Returns
    /// The owning domain adapter, or `None` when no domain registered the kind.
    fn lookup(&self, kind: &ProjectionKind) -> Option<&Arc<dyn ProjectionStyleAdapter>> {
        self.entries
            .iter()
            .find(|entry| entry.kind == kind.as_str())
            .map(|entry| &entry.adapter)
    }
}

impl ProjectionStyleAdapter for KindDispatchStyleAdapter {
    /// Returns the schema declared by the domain that owns the kind.
    ///
    /// - `self`: Dispatcher answering the query.
    /// - `kind`: Projection kind being inspected.
    ///
    /// # Returns
    /// The domain schema when the kind is registered and declares one.
    fn style_schema(&self, kind: &ProjectionKind) -> Option<StyleSchema> {
        self.lookup(kind)
            .and_then(|adapter| adapter.style_schema(kind))
    }

    /// Supplies the default style of the domain that owns the kind.
    ///
    /// - `self`: Dispatcher answering the query.
    /// - `kind`: Projection kind being rendered.
    ///
    /// # Returns
    /// The owning domain's exact default style.
    ///
    /// # Errors
    /// Returns an unsupported diagnostic when no domain registered the kind.
    fn default_style(
        &self,
        kind: &ProjectionKind,
    ) -> Result<ResolvedStyle, Box<ProjectionDiagnostic>> {
        match self.lookup(kind) {
            Some(adapter) => adapter.default_style(kind),
            None => Err(Box::new(unsupported_style_diagnostic(
                kind,
                "style-domain-not-registered",
                "no domain style adapter is registered for this projection kind",
            ))),
        }
    }

    /// Normalizes style input through the domain that owns the kind.
    ///
    /// - `self`: Dispatcher performing normalization.
    /// - `kind`: Projection kind whose domain owns the fields.
    /// - `input`: Style input to canonicalize.
    ///
    /// # Returns
    /// The domain's canonical style bytes.
    ///
    /// # Errors
    /// Returns the domain's diagnostic, or an unsupported diagnostic when the
    /// kind has no registered domain.
    fn normalize(
        &self,
        kind: &ProjectionKind,
        input: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        match self.lookup(kind) {
            Some(adapter) => adapter.normalize(kind, input),
            None => Err(Box::new(unsupported_style_diagnostic(
                kind,
                "style-domain-not-registered",
                "no domain style adapter is registered for this projection kind",
            ))),
        }
    }

    /// Merges two styles through the domain that owns the kind.
    ///
    /// - `self`: Dispatcher performing the merge.
    /// - `kind`: Projection kind whose domain owns merge meaning.
    /// - `base`: Lower-precedence canonical style.
    /// - `overlay`: Higher-precedence canonical style.
    ///
    /// # Returns
    /// The domain-merged canonical style.
    ///
    /// # Errors
    /// Returns the domain's diagnostic, or an unsupported diagnostic when the
    /// kind has no registered domain.
    fn merge(
        &self,
        kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> Result<StyleValue, Box<ProjectionDiagnostic>> {
        match self.lookup(kind) {
            Some(adapter) => adapter.merge(kind, base, overlay),
            None => Err(Box::new(unsupported_style_diagnostic(
                kind,
                "style-domain-not-registered",
                "no domain style adapter is registered for this projection kind",
            ))),
        }
    }

    /// Classifies fidelity through the domain that owns the kind.
    ///
    /// - `self`: Dispatcher owning fidelity.
    /// - `kind`: Projection kind being rendered.
    /// - `style`: Exact resolved style a renderer would consume.
    /// - `purpose`: Why the projection is being produced.
    ///
    /// # Returns
    /// The owning domain's assessment, or a conservative assessment when no
    /// domain registered the kind.
    fn classify(
        &self,
        kind: &ProjectionKind,
        style: &ResolvedStyle,
        purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        match self.lookup(kind) {
            Some(adapter) => adapter.classify(kind, style, purpose),
            None => FidelityAssessment::conservative(
                "no domain style adapter is registered for this projection kind",
            ),
        }
    }
}

//! Shared application ownership of projection style selection and resolution.
//!
//! Product surfaces present style intent; this module decides which identified
//! sources apply for a purpose and produces the exact renderer-consumable style
//! plus its provenance and fidelity. Style meaning itself stays with the domain
//! adapter installed by the host composition.

pub mod backend;
pub mod convention;
pub mod dispatch;
pub mod resolver;
pub mod store;

use std::sync::Arc;

use sr_projection::{
    FidelityAssessment, ProjectionKind, ProjectionPurpose, ProjectionStyleAdapter, ResolvedStyle,
    StyleSchema, StyleValue, unsupported_style_diagnostic,
};

pub use backend::{
    EffectiveProjectionStyle, ProjectionStyleBackend, ProjectionStyleFidelityRequest,
    ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest,
};
pub use convention::{RepositoryConventionSource, StaticRepositoryConvention};
pub use dispatch::{KindDispatchStyleAdapter, RegisteredStyleAdapter};
pub use resolver::{
    StyleResolution, StyleResolutionRequest, resolve_effective_style, resolve_fidelity,
};
pub use store::{
    InMemoryProjectionStyleStore, ProjectionStyleStore, StoredStyle, StyleEntryKey, StyleScopeKind,
};

/// Fixed-style adapter used while a domain has not implemented style discovery.
///
/// It resolves to exactly one canonical default style, classifies conservatively
/// as read-only lossy, and refuses normalization and merging. Refusing merge is
/// deliberate: a shared resolver must never fall back to generic last-write-wins
/// layering for a domain that has not declared merge meaning.
///
/// # Public members
/// - None.
///
/// # Private members
/// - None.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompatibilityStyleAdapter;

impl ProjectionStyleAdapter for CompatibilityStyleAdapter {
    /// Declares no schema because the domain exposes none yet.
    ///
    /// - `self`: Adapter answering the query.
    /// - `_kind`: Kind being inspected.
    ///
    /// # Returns
    /// Always `None`.
    fn style_schema(&self, _kind: &ProjectionKind) -> Option<StyleSchema> {
        None
    }

    /// Provides the single fixed canonical default style.
    ///
    /// - `self`: Adapter owning the default.
    /// - `_kind`: Kind being rendered.
    ///
    /// # Returns
    /// The fixed empty canonical style, always.
    ///
    /// # Errors
    /// Never fails for this adapter.
    fn default_style(
        &self,
        _kind: &ProjectionKind,
    ) -> sr_projection::ProjectionResult<ResolvedStyle> {
        Ok(ResolvedStyle::inline(StyleValue::new(Vec::new())))
    }

    /// Refuses normalization because no domain owns it yet.
    ///
    /// - `self`: Adapter that would normalize.
    /// - `kind`: Kind the style renders.
    /// - `_input`: Style input.
    ///
    /// # Returns
    /// Always a structured unsupported diagnostic.
    ///
    /// # Errors
    /// Returns an unsupported-operation diagnostic.
    fn normalize(
        &self,
        kind: &ProjectionKind,
        _input: &StyleValue,
    ) -> sr_projection::ProjectionResult<StyleValue> {
        Err(Box::new(unsupported_style_diagnostic(
            kind,
            "style-normalization-unsupported",
            "this projection adapter has not declared style normalization yet",
        )))
    }

    /// Refuses merging because the domain owns no merge meaning yet.
    ///
    /// - `self`: Adapter that would merge.
    /// - `kind`: Kind the style renders.
    /// - `_base`: Lower-precedence style.
    /// - `_overlay`: Higher-precedence style.
    ///
    /// # Returns
    /// Always a structured unsupported diagnostic.
    ///
    /// # Errors
    /// Returns an unsupported-operation diagnostic.
    fn merge(
        &self,
        kind: &ProjectionKind,
        _base: &StyleValue,
        _overlay: &StyleValue,
    ) -> sr_projection::ProjectionResult<StyleValue> {
        Err(Box::new(unsupported_style_diagnostic(
            kind,
            "style-merge-unsupported",
            "this projection adapter has not declared style merge semantics yet",
        )))
    }

    /// Classifies conservatively until the domain proves reconciliation safety.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `kind`: Kind being rendered.
    /// - `_style`: Exact resolved style.
    /// - `_purpose`: Why the projection is produced.
    ///
    /// # Returns
    /// A read-only lossy assessment.
    fn classify(
        &self,
        kind: &ProjectionKind,
        _style: &ResolvedStyle,
        _purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        let _ = kind;
        FidelityAssessment::conservative(
            "this projection adapter has not classified style fidelity yet",
        )
    }
}

/// Installed projection style ownership for one application service.
///
/// # Public members
/// - `adapter`: Domain style adapter owning default, normalize, merge, fidelity.
/// - `store`: Configuration persistence boundary for each scope.
/// - `conventions`: Ordered repository convention sources.
/// - `new`: Builds a configuration from explicit parts.
/// - `with_conventions`: Adds convention sources.
///
/// # Private members
/// - None.
#[derive(Clone)]
pub struct ProjectionStyleConfiguration {
    /// Domain style adapter owning default, normalize, merge, fidelity.
    pub adapter: Arc<dyn ProjectionStyleAdapter>,
    /// Configuration persistence boundary for each scope.
    pub store: Arc<dyn ProjectionStyleStore>,
    /// Ordered repository convention sources.
    pub conventions: Vec<Arc<dyn RepositoryConventionSource>>,
}

impl ProjectionStyleConfiguration {
    /// Builds a configuration from explicit ownership parts.
    ///
    /// - `adapter`: Domain style adapter.
    /// - `store`: Configuration persistence boundary.
    ///
    /// # Returns
    /// The configuration with no convention sources installed.
    #[must_use]
    pub fn new(
        adapter: Arc<dyn ProjectionStyleAdapter>,
        store: Arc<dyn ProjectionStyleStore>,
    ) -> Self {
        Self {
            adapter,
            store,
            conventions: Vec::new(),
        }
    }

    /// Adds ordered repository convention sources.
    ///
    /// - `self`: Configuration being extended.
    /// - `conventions`: Sources in application order.
    ///
    /// # Returns
    /// The configuration with the sources appended.
    #[must_use]
    pub fn with_conventions(
        mut self,
        conventions: Vec<Arc<dyn RepositoryConventionSource>>,
    ) -> Self {
        self.conventions.extend(conventions);
        self
    }
}

impl Default for ProjectionStyleConfiguration {
    /// Builds the unmigrated compatibility configuration.
    ///
    /// # Returns
    /// A configuration using the fixed compatibility style adapter and an
    /// in-memory store, so style reads work before any domain migrates.
    fn default() -> Self {
        Self::new(
            Arc::new(CompatibilityStyleAdapter),
            Arc::new(InMemoryProjectionStyleStore::new()),
        )
    }
}

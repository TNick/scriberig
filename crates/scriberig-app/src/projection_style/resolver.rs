//! Resolves one exact projection style from identified sources per purpose.

use std::sync::Arc;

use sr_projection::{
    FidelityAssessment, ProjectionDiagnostic, ProjectionKind, ProjectionPurpose,
    ProjectionStyleAdapter, ResolvedStyle, ResolvedStyleProvenance, StyleRef, StyleSource,
    StyleSourceKind, StyleValue,
};

use crate::ApplicationError;

use super::convention::RepositoryConventionSource;
use super::store::{ProjectionStyleStore, StyleEntryKey, StyleScopeKind};

/// Names everything resolution needs to pick one exact style.
///
/// Identity fields are supplied by the caller because the shared layer must not
/// infer project, workspace, or view scope from ambient process state.
///
/// # Public members
/// - `projection_kind`: Kind being projected.
/// - `project_id`: Stable project identity whose policy and conventions apply.
/// - `workspace_id`: Stable workspace identity whose materialization policy applies.
/// - `purpose`: Why the projection is being produced.
/// - `principal`: Trusted principal for personal sources, when the caller has one.
/// - `view_id`: Specific view session a view override may apply to.
/// - `explicit_personal`: Personal style named explicitly by an export caller.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleResolutionRequest {
    /// Kind being projected.
    pub projection_kind: ProjectionKind,
    /// Stable project identity whose policy and conventions apply.
    pub project_id: String,
    /// Stable workspace identity whose materialization policy applies.
    pub workspace_id: String,
    /// Why the projection is being produced.
    pub purpose: ProjectionPurpose,
    /// Trusted principal for personal sources, when the caller has one.
    pub principal: Option<String>,
    /// Specific view session a view override may apply to.
    pub view_id: Option<String>,
    /// Personal style named explicitly by an export caller.
    pub explicit_personal: Option<StyleValue>,
}

impl StyleResolutionRequest {
    /// Builds a resolution request for one purpose and scope triple.
    ///
    /// - `projection_kind`: Kind being projected.
    /// - `project_id`: Stable project identity.
    /// - `workspace_id`: Stable workspace identity.
    /// - `purpose`: Why the projection is being produced.
    ///
    /// # Returns
    /// The request with no personal sources selected.
    #[must_use]
    pub fn new(
        projection_kind: ProjectionKind,
        project_id: impl Into<String>,
        workspace_id: impl Into<String>,
        purpose: ProjectionPurpose,
    ) -> Self {
        Self {
            projection_kind,
            project_id: project_id.into(),
            workspace_id: workspace_id.into(),
            purpose,
            principal: None,
            view_id: None,
            explicit_personal: None,
        }
    }
}

/// One resolved style with the evidence that produced it.
///
/// # Public members
/// - `provenance`: Exact style plus the ordered applied source chain.
/// - `fidelity`: Adapter classification for that exact resolved style.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleResolution {
    /// Exact style plus the ordered applied source chain.
    pub provenance: ResolvedStyleProvenance,
    /// Adapter classification for that exact resolved style.
    pub fidelity: FidelityAssessment,
}

/// Resolves the effective style for one purpose using adapter-owned meaning.
///
/// - `adapter`: Domain style adapter owning default, normalize, merge, fidelity.
/// - `store`: Style configuration persistence boundary for each scope.
/// - `conventions`: Ordered repository convention sources.
/// - `request`: Scope, purpose, and explicitly named personal sources.
///
/// Source order is fixed per purpose. Interactive view applies domain default,
/// repository conventions, project policy, user preference, then view override.
/// Shared materialization stops at workspace policy and never reads personal
/// sources. Explicit export applies personal state only when the caller named
/// it, and records that choice in the applied chain.
///
/// # Returns
/// The exact resolved style, its applied provenance chain, and fidelity.
///
/// # Errors
/// Returns `ApplicationError::InvalidRequest` when a personal style is named
/// outside the export purpose, `ApplicationError::UnsupportedOperation` when the
/// adapter declares no default style, `ApplicationError::SchemaViolation` when
/// the domain rejects a style input or merge, and store errors for reads.
pub fn resolve_effective_style(
    adapter: &dyn ProjectionStyleAdapter,
    store: &dyn ProjectionStyleStore,
    conventions: &[Arc<dyn RepositoryConventionSource>],
    request: &StyleResolutionRequest,
) -> Result<StyleResolution, ApplicationError> {
    if request.explicit_personal.is_some() && request.purpose != ProjectionPurpose::ExplicitExport {
        return Err(ApplicationError::InvalidRequest {
            operation: "projection style resolve".to_owned(),
            message: "an explicit personal style requires the explicit-export purpose".to_owned(),
        });
    }

    let default = adapter
        .default_style(&request.projection_kind)
        .map_err(|diagnostic| style_error("projection style default", diagnostic))?;
    let mut layered = LayeredStyle::new(adapter, store, request, default.value().clone());
    layered.record(
        StyleSourceKind::DomainDefault,
        "adapter-domain-default",
        default.value(),
    );

    for source in conventions {
        if let Some(value) =
            source.convention_style(&request.project_id, &request.projection_kind)?
        {
            layered.apply_value(
                &value,
                StyleSourceKind::RepositoryConvention,
                "repository-convention",
            )?;
        }
    }
    layered.apply_scope(
        StyleScopeKind::Project,
        &request.project_id,
        StyleSourceKind::ProjectPolicy,
    )?;

    if request.purpose == ProjectionPurpose::InteractiveView {
        if let Some(principal) = request.principal.as_deref() {
            layered.apply_scope(
                StyleScopeKind::User,
                principal,
                StyleSourceKind::UserPreference,
            )?;
        }
        if let Some(view_id) = request.view_id.as_deref() {
            layered.apply_scope(StyleScopeKind::View, view_id, StyleSourceKind::ViewOverride)?;
        }
    } else {
        layered.apply_scope(
            StyleScopeKind::Workspace,
            &request.workspace_id,
            StyleSourceKind::WorkspaceMaterializationPolicy,
        )?;
    }

    if let Some(explicit) = request.explicit_personal.as_ref() {
        layered.apply_value(
            explicit,
            StyleSourceKind::ExplicitExportOverride,
            "caller-explicit-export",
        )?;
    }

    layered.finish()
}

/// Classifies fidelity for one already resolved style.
///
/// - `adapter`: Domain style adapter owning fidelity.
/// - `projection_kind`: Kind being projected.
/// - `resolved`: Exact resolved style a renderer would consume.
/// - `purpose`: Why the projection is being produced.
///
/// Fidelity is answered only for an exact resolved style, never for a projection
/// kind alone, so a caller cannot infer editability from a specification flag.
///
/// # Returns
/// The adapter's fidelity assessment.
#[must_use]
pub fn resolve_fidelity(
    adapter: &dyn ProjectionStyleAdapter,
    projection_kind: &ProjectionKind,
    resolved: &ResolvedStyle,
    purpose: ProjectionPurpose,
) -> FidelityAssessment {
    adapter.classify(projection_kind, resolved, purpose)
}

/// Accumulates one style while recording the sources that contributed to it.
///
/// # Private members
/// - `adapter`: Domain adapter owning normalize and merge meaning.
/// - `store`: Configuration persistence boundary.
/// - `request`: Request naming the kind and scopes being resolved.
/// - `current`: Style accumulated from the sources applied so far.
/// - `applied`: Ordered provenance chain.
struct LayeredStyle<'a> {
    /// Domain adapter owning normalize and merge meaning.
    adapter: &'a dyn ProjectionStyleAdapter,
    /// Configuration persistence boundary.
    store: &'a dyn ProjectionStyleStore,
    /// Request naming the kind and scopes being resolved.
    request: &'a StyleResolutionRequest,
    /// Style accumulated from the sources applied so far.
    current: StyleValue,
    /// Ordered provenance chain.
    applied: Vec<StyleSource>,
}

impl<'a> LayeredStyle<'a> {
    /// Starts one accumulation from the domain default value.
    ///
    /// - `adapter`: Domain adapter owning style meaning.
    /// - `store`: Configuration persistence boundary.
    /// - `request`: Request naming the kind and scopes.
    /// - `current`: Starting style value.
    ///
    /// # Returns
    /// The empty accumulation carrying the starting value.
    fn new(
        adapter: &'a dyn ProjectionStyleAdapter,
        store: &'a dyn ProjectionStyleStore,
        request: &'a StyleResolutionRequest,
        current: StyleValue,
    ) -> Self {
        Self {
            adapter,
            store,
            request,
            current,
            applied: Vec::new(),
        }
    }

    /// Records one applied source in the provenance chain.
    ///
    /// - `kind`: Source class contributed.
    /// - `identity`: Stable identity of the contributing source.
    /// - `value`: Canonical value that source contributed.
    fn record(&mut self, kind: StyleSourceKind, identity: &str, value: &StyleValue) {
        self.applied.push(StyleSource::new(
            kind,
            identity.to_owned(),
            None,
            value.clone(),
        ));
    }

    /// Reads one scope's stored setting and applies it when one exists.
    ///
    /// - `scope`: Configuration scope to read.
    /// - `identity`: Scope identity whose setting applies.
    /// - `source_kind`: Source class contributed by this scope.
    ///
    /// # Returns
    /// Unit once the setting, if any, has been merged into the accumulation.
    ///
    /// # Errors
    /// Returns a schema violation when the domain rejects the stored style, and
    /// store errors for reads.
    fn apply_scope(
        &mut self,
        scope: StyleScopeKind,
        identity: &str,
        source_kind: StyleSourceKind,
    ) -> Result<(), ApplicationError> {
        let Some(entry) = self.store.read(&StyleEntryKey::new(
            scope,
            identity,
            self.request.projection_kind.clone(),
        ))?
        else {
            return Ok(());
        };
        self.apply_value(&entry.value, source_kind, identity)
    }

    /// Normalizes and merges one candidate value into the accumulation.
    ///
    /// - `value`: Raw candidate contributed by this source.
    /// - `source_kind`: Source class being applied.
    /// - `identity`: Stable identity of the contributing source.
    ///
    /// # Returns
    /// Unit once the candidate has been merged into the accumulation.
    ///
    /// # Errors
    /// Returns a schema violation when the domain rejects the input or merge.
    fn apply_value(
        &mut self,
        value: &StyleValue,
        source_kind: StyleSourceKind,
        identity: &str,
    ) -> Result<(), ApplicationError> {
        let normalized = self
            .adapter
            .normalize(&self.request.projection_kind, value)
            .map_err(|diagnostic| style_error("projection style normalize", diagnostic))?;
        self.current = self
            .adapter
            .merge(&self.request.projection_kind, &self.current, &normalized)
            .map_err(|diagnostic| style_error("projection style merge", diagnostic))?;
        self.record(source_kind, identity, &normalized);
        Ok(())
    }

    /// Resolves the accumulated style and classifies fidelity for it.
    ///
    /// The accumulated value is already canonical: each applied input was
    /// normalized through the domain adapter before merging, and the adapter's
    /// merge output is canonical by contract. Re-normalizing here would force an
    /// adapter that supplies only a fixed default style to declare normalization
    /// it has not defined.
    ///
    /// # Returns
    /// The resolved style, its provenance chain, and the adapter's fidelity.
    ///
    /// # Errors
    /// Returns a schema violation when the resolved reference disagrees with the
    /// accumulated value.
    fn finish(self) -> Result<StyleResolution, ApplicationError> {
        let canonical = self.current;
        let schema = self
            .adapter
            .style_schema(&self.request.projection_kind)
            .map(|declared| declared.reference);
        let resolved = ResolvedStyle::new(StyleRef::Value(canonical.clone()), canonical, schema)
            .map_err(|error| ApplicationError::SchemaViolation {
                message: format!("resolved projection style is inconsistent: {error}"),
            })?;
        let fidelity = self.adapter.classify(
            &self.request.projection_kind,
            &resolved,
            self.request.purpose,
        );
        Ok(StyleResolution {
            provenance: ResolvedStyleProvenance::new(resolved, self.applied),
            fidelity,
        })
    }
}

/// Converts a projection style diagnostic into an application error.
///
/// - `operation`: Style operation that failed.
/// - `diagnostic`: Structured adapter diagnostic.
///
/// # Returns
/// A schema violation when the domain rejected input, an unsupported-operation
/// error when the adapter declares no meaning for the request, otherwise an
/// internal error naming the operation and adapter code.
fn style_error(operation: &str, diagnostic: Box<ProjectionDiagnostic>) -> ApplicationError {
    if diagnostic.kind == sr_projection::DiagnosticKind::InvalidConfiguration {
        return ApplicationError::SchemaViolation {
            message: diagnostic
                .message
                .unwrap_or_else(|| format!("{operation} produced an invalid style")),
        };
    }
    if diagnostic.kind == sr_projection::DiagnosticKind::UnsupportedOperation {
        return ApplicationError::UnsupportedOperation {
            operation: operation.to_owned(),
        };
    }
    ApplicationError::Internal {
        message: format!("{operation} failed: {}", diagnostic.detail.code),
    }
}

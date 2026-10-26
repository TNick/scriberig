//! Declares the shared application surface for projection style semantics.

use sr_projection::{
    FidelityAssessment, ProjectionKind, ProjectionPurpose, ResolvedStyle, StyleSource, StyleValue,
};

use crate::{ApplicationContext, ApplicationError};

use super::resolver::{StyleResolution, StyleResolutionRequest};
use super::store::StoredStyle;

/// Asks which style schema one projection kind advertises.
///
/// # Public members
/// - `projection_kind`: Kind whose schema is requested.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionStyleSchemaRequest {
    /// Kind whose schema is requested.
    pub projection_kind: ProjectionKind,
}

/// Asks for fidelity of one already resolved style.
///
/// # Public members
/// - `projection_kind`: Kind that would be rendered.
/// - `resolved`: Exact resolved style a renderer would consume.
/// - `purpose`: Why the projection is being produced.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionStyleFidelityRequest {
    /// Kind that would be rendered.
    pub projection_kind: ProjectionKind,
    /// Exact resolved style a renderer would consume.
    pub resolved: ResolvedStyle,
    /// Why the projection is being produced.
    pub purpose: ProjectionPurpose,
}

/// Sets or clears one style setting in a named configuration scope.
///
/// One request type covers the personal, view, project, and workspace scopes;
/// each operation names its own scope identifier. A `None` style clears the
/// setting. None of these writes touch canonical semantic state.
///
/// # Public members
/// - `projection_kind`: Kind the setting applies to.
/// - `scope_id`: Stable identity of the scope being written.
/// - `expected_revision`: Revision observed by the caller, when it insists on one.
/// - `style`: New canonical style input, or `None` to clear.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionStyleSettingRequest {
    /// Kind the setting applies to.
    pub projection_kind: ProjectionKind,
    /// Stable identity of the scope being written.
    pub scope_id: String,
    /// Revision observed by the caller, when it insists on one.
    pub expected_revision: Option<u64>,
    /// New canonical style input, or `None` to clear.
    pub style: Option<StyleValue>,
}

impl ProjectionStyleSettingRequest {
    /// Builds one setting write.
    ///
    /// - `projection_kind`: Kind the setting applies to.
    /// - `scope_id`: Stable identity of the scope being written.
    /// - `style`: New canonical style input, or `None` to clear.
    ///
    /// # Returns
    /// The request without a revision precondition.
    #[must_use]
    pub fn new(
        projection_kind: ProjectionKind,
        scope_id: impl Into<String>,
        style: Option<StyleValue>,
    ) -> Self {
        Self {
            projection_kind,
            scope_id: scope_id.into(),
            expected_revision: None,
            style,
        }
    }
}

/// The effective style one caller would render with, plus its evidence.
///
/// # Public members
/// - `purpose`: Purpose the resolution was performed for.
/// - `resolved`: Exact style a renderer must consume.
/// - `applied`: Ordered sources that contributed to the result.
/// - `fidelity`: Adapter classification for that exact style.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveProjectionStyle {
    /// Purpose the resolution was performed for.
    pub purpose: ProjectionPurpose,
    /// Exact style a renderer must consume.
    pub resolved: ResolvedStyle,
    /// Ordered sources that contributed to the result.
    pub applied: Vec<StyleSource>,
    /// Adapter classification for that exact style.
    pub fidelity: FidelityAssessment,
}

impl EffectiveProjectionStyle {
    /// Builds one effective-style view from a resolution result.
    ///
    /// - `purpose`: Purpose the resolution was performed for.
    /// - `resolution`: Resolution outcome carrying provenance and fidelity.
    ///
    /// # Returns
    /// The effective-style view.
    #[must_use]
    pub fn from_resolution(purpose: ProjectionPurpose, resolution: StyleResolution) -> Self {
        Self {
            purpose,
            resolved: resolution.provenance.resolved,
            applied: resolution.provenance.applied,
            fidelity: resolution.fidelity,
        }
    }
}

/// Exposes projection style semantics to every product surface through one owner.
///
/// The same operations must be reachable embedded and remotely, so no surface may
/// grow its own precedence rules, default table, or formatter.
///
/// # Public members
/// - `projection_style_schema`: Adapter style schema for one kind.
/// - `projection_style_effective`: Effective style, provenance, and fidelity.
/// - `projection_style_fidelity`: Fidelity for one exact resolved style.
/// - `set_projection_style_user_preference`: Set or clear a personal preference.
/// - `set_projection_style_view_override`: Set or clear one view override.
/// - `set_projection_style_project_policy`: Set or clear shared project policy.
/// - `set_projection_style_materialization_policy`: Set or clear workspace policy.
///
/// # Private members
/// - None.
pub trait ProjectionStyleBackend {
    /// Reports the adapter style schema for one projection kind.
    ///
    /// - `self`: Application surface answering the query.
    /// - `_context`: Caller context; reads require no principal.
    /// - `request`: Kind whose schema is requested.
    ///
    /// # Returns
    /// Structured schema metadata when the adapter declares a style schema.
    ///
    /// # Errors
    /// Returns `ApplicationError::UnsupportedOperation` when the installed
    /// projection style adapter cannot answer schema queries.
    fn projection_style_schema(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleSchemaRequest,
    ) -> Result<Option<sr_projection::StyleSchema>, ApplicationError>;

    /// Resolves the effective style for one purpose and scope set.
    ///
    /// - `self`: Application surface answering the query.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, scopes, purpose, and any explicit export source.
    ///
    /// # Returns
    /// Exact style, applied provenance, and fidelity.
    ///
    /// # Errors
    /// Returns a schema violation when the domain rejects a style, and store
    /// errors when configuration cannot be read.
    fn projection_style_effective(
        &self,
        context: &ApplicationContext,
        request: &StyleResolutionRequest,
    ) -> Result<EffectiveProjectionStyle, ApplicationError>;

    /// Reports fidelity for one exact resolved style.
    ///
    /// - `self`: Application surface answering the query.
    /// - `_context`: Caller context; reads require no principal.
    /// - `request`: Kind, exact resolved style, and purpose.
    ///
    /// # Returns
    /// The adapter's fidelity assessment for that exact style.
    ///
    /// # Errors
    /// Never fails for an installed adapter; the result type keeps the surface
    /// uniform with the other style operations.
    fn projection_style_fidelity(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleFidelityRequest,
    ) -> Result<FidelityAssessment, ApplicationError>;

    /// Sets or clears one personal style preference.
    ///
    /// - `self`: Application surface performing the write.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, optional revision, and new style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when it was cleared.
    ///
    /// # Errors
    /// Returns permission-denied without a principal, and a semantic conflict when
    /// the caller's revision is stale.
    fn set_projection_style_user_preference(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError>;

    /// Sets or clears one view-session style override.
    ///
    /// - `self`: Application surface performing the write.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, view identity, optional revision, and new style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when it was cleared.
    ///
    /// # Errors
    /// Returns permission-denied without a principal, and a semantic conflict when
    /// the caller's revision is stale.
    fn set_projection_style_view_override(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError>;

    /// Sets or clears shared project style policy.
    ///
    /// - `self`: Application surface performing the write.
    /// - `context`: Caller context for authorization.
    /// - `request`: Kind, project identity, optional revision, and new style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when it was cleared.
    ///
    /// # Errors
    /// Returns a semantic conflict when the caller's revision is stale.
    fn set_projection_style_project_policy(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError>;

    /// Sets or clears workspace materialization style policy.
    ///
    /// - `self`: Application surface performing the write.
    /// - `context`: Caller context for authorization.
    /// - `request`: Kind, workspace identity, optional revision, style, or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when it was cleared.
    ///
    /// # Errors
    /// Returns a semantic conflict when the caller's revision is stale.
    fn set_projection_style_materialization_policy(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError>;
}

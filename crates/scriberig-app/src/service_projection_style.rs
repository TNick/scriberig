//! Implements shared projection style operations for every product surface.

use crate::projection_style::store::StyleScopeKind;
use crate::projection_style::{
    EffectiveProjectionStyle, ProjectionStyleBackend, ProjectionStyleFidelityRequest,
    ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest, resolve_effective_style,
    resolve_fidelity,
};
use crate::projection_style::{StoredStyle, StyleEntryKey, StyleResolutionRequest};
use crate::{ApplicationContext, ApplicationError, ApplicationService};

impl<B> ProjectionStyleBackend for ApplicationService<B> {
    /// Reports the installed adapter's style schema for one projection kind.
    ///
    /// - `self`: Application service answering the query.
    /// - `_context`: Reads need no principal.
    /// - `request`: Kind whose schema is requested.
    ///
    /// # Returns
    /// Structured schema metadata when the installed adapter declares one.
    ///
    /// # Errors
    /// Never fails; the trait documents unsupported adapters as the failure case.
    fn projection_style_schema(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleSchemaRequest,
    ) -> Result<Option<sr_projection::StyleSchema>, ApplicationError> {
        Ok(self
            .projection_style
            .adapter
            .style_schema(&request.projection_kind))
    }

    /// Resolves the effective style, provenance, and fidelity for one purpose.
    ///
    /// - `self`: Application service answering the query.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, scopes, purpose, and any explicit export source.
    ///
    /// A personal principal is attached only for the interactive purpose, so a
    /// shared materialization query cannot pick up whoever happens to be logged
    /// in.
    ///
    /// # Returns
    /// The effective style view for the requested purpose.
    ///
    /// # Errors
    /// Returns a schema violation when the domain rejects a configured style.
    fn projection_style_effective(
        &self,
        context: &ApplicationContext,
        request: &StyleResolutionRequest,
    ) -> Result<EffectiveProjectionStyle, ApplicationError> {
        let mut scoped = request.clone();
        if scoped.purpose == sr_projection::ProjectionPurpose::InteractiveView {
            scoped.principal = personal_principal(context).map(str::to_owned);
        } else {
            scoped.principal = None;
            scoped.view_id = None;
        }
        let resolution = resolve_effective_style(
            self.projection_style.adapter.as_ref(),
            self.projection_style.store.as_ref(),
            &self.projection_style.conventions,
            &scoped,
        )?;
        Ok(EffectiveProjectionStyle::from_resolution(
            request.purpose,
            resolution,
        ))
    }

    /// Reports fidelity for one exact resolved style.
    ///
    /// - `self`: Application service answering the query.
    /// - `_context`: Reads need no principal.
    /// - `request`: Kind, exact resolved style, and purpose.
    ///
    /// # Returns
    /// The adapter's assessment for that exact style.
    ///
    /// # Errors
    /// Never fails for an installed adapter.
    fn projection_style_fidelity(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleFidelityRequest,
    ) -> Result<sr_projection::FidelityAssessment, ApplicationError> {
        Ok(resolve_fidelity(
            self.projection_style.adapter.as_ref(),
            &request.projection_kind,
            &request.resolved,
            request.purpose,
        ))
    }

    /// Sets or clears the calling principal's style preference.
    ///
    /// - `self`: Application service performing the write.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, optional revision, and new style or clear.
    ///
    /// The stored scope is always the trusted principal, never a caller-supplied
    /// identity, so one principal cannot write another's preference.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when cleared.
    ///
    /// # Errors
    /// Returns permission-denied when no principal is trusted on the context.
    fn set_projection_style_user_preference(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError> {
        let principal = personal_principal(context)
            .ok_or_else(|| permission_denied("set projection style preference"))?
            .to_owned();
        self.write_setting(
            &StyleEntryKey::new(
                StyleScopeKind::User,
                principal,
                request.projection_kind.clone(),
            ),
            request,
        )
    }

    /// Sets or clears one view-session style override.
    ///
    /// - `self`: Application service performing the write.
    /// - `context`: Caller context supplying the trusted personal principal.
    /// - `request`: Kind, view identity, optional revision, and style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when cleared.
    ///
    /// # Errors
    /// Returns permission-denied when no principal is trusted on the context, and
    /// an invalid-request error when the view identity is empty.
    fn set_projection_style_view_override(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError> {
        if personal_principal(context).is_none() {
            return Err(permission_denied("set projection style view override"));
        }
        if request.scope_id.trim().is_empty() {
            return Err(ApplicationError::InvalidRequest {
                operation: "set projection style view override".to_owned(),
                message: "a view override requires a view identity".to_owned(),
            });
        }
        self.write_setting(
            &StyleEntryKey::new(
                StyleScopeKind::View,
                &request.scope_id,
                request.projection_kind.clone(),
            ),
            request,
        )
    }

    /// Sets or clears shared project style policy.
    ///
    /// - `self`: Application service performing the write.
    /// - `context`: Caller context supplying an audit identity.
    /// - `request`: Kind, project identity, optional revision, and style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when cleared.
    ///
    /// # Errors
    /// Returns permission-denied when the context carries no identity to audit the
    /// shared configuration change under.
    fn set_projection_style_project_policy(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError> {
        require_audit_identity(context)?;
        self.write_setting(
            &StyleEntryKey::new(
                StyleScopeKind::Project,
                &request.scope_id,
                request.projection_kind.clone(),
            ),
            request,
        )
    }

    /// Sets or clears workspace materialization style policy.
    ///
    /// - `self`: Application service performing the write.
    /// - `context`: Caller context supplying an audit identity.
    /// - `request`: Kind, workspace identity, optional revision, and style or clear.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when cleared.
    ///
    /// # Errors
    /// Returns permission-denied when the context carries no identity to audit the
    /// shared configuration change under.
    fn set_projection_style_materialization_policy(
        &self,
        context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError> {
        require_audit_identity(context)?;
        self.write_setting(
            &StyleEntryKey::new(
                StyleScopeKind::Workspace,
                &request.scope_id,
                request.projection_kind.clone(),
            ),
            request,
        )
    }
}

impl<B> ApplicationService<B> {
    /// Applies one style setting write or clear to a resolved configuration key.
    ///
    /// - `self`: Application service owning the configuration store.
    /// - `key`: Resolved configuration key to write.
    /// - `request`: Request carrying the optional revision and new style or clear.
    ///
    /// Style settings are presentation and policy configuration. Writing one never
    /// opens a candidate, migration, or change operation against canonical state.
    ///
    /// # Returns
    /// The authoritative stored setting, or `None` when cleared.
    ///
    /// # Errors
    /// Returns a semantic conflict when the caller's revision is stale.
    fn write_setting(
        &self,
        key: &StyleEntryKey,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<Option<StoredStyle>, ApplicationError> {
        match request.style.clone() {
            Some(style_value) => self
                .projection_style
                .store
                .compare_and_swap(key, request.expected_revision, style_value)
                .map(Some),
            None => self
                .projection_style
                .store
                .clear(key, request.expected_revision)
                .map(|()| None),
        }
    }
}

/// Reads the trusted personal principal from one application context.
///
/// - `context`: Caller context to inspect.
///
/// # Returns
/// The personal principal when the trusted runtime supplied one.
///
fn personal_principal(context: &ApplicationContext) -> Option<&str> {
    context.user_preferences_principal.as_deref().or_else(|| {
        context
            .principal
            .as_ref()
            .map(|principal| principal.id.as_str())
    })
}

/// Builds a permission-denied error for one style operation.
///
/// - `action`: Style action that lacked authority.
///
/// # Returns
/// The permission-denied application error.
fn permission_denied(action: &str) -> ApplicationError {
    ApplicationError::PermissionDenied {
        action: action.to_owned(),
        scope: "projection style configuration".to_owned(),
    }
}

/// Requires an identity to audit a shared style configuration change under.
///
/// - `context`: Caller context to inspect.
///
/// # Returns
/// Unit when an identity is present.
///
/// # Errors
/// Returns permission-denied when no identity is available for audit.
fn require_audit_identity(context: &ApplicationContext) -> Result<(), ApplicationError> {
    if context.principal.is_some() || context.user_preferences_principal.is_some() {
        return Ok(());
    }
    Err(permission_denied("change shared projection style policy"))
}

//! Resolves an explicit render style selection before any provider renders.
//!
//! Providers hold no style configuration, so the application layer is the only
//! place a configured style may be resolved. This module turns a caller's
//! selection into the exact value a render must apply and records which sources
//! produced it, so a provider can refuse anything unresolved.

use crate::projection_style::{StyleResolutionRequest, resolve_effective_style};
use crate::request::RenderStyleSelection;
use crate::service::service_projection_style::personal_principal;
use crate::{ApplicationContext, ApplicationError, ApplicationService};

impl<B> ApplicationService<B> {
    /// Resolves one render style selection into the exact style a render applies.
    ///
    /// A configured selection is resolved through the installed domain style
    /// adapter and configuration store with the caller's purpose; personal
    /// sources attach only for the interactive and export purposes, so a shared
    /// materialization render cannot pick up whoever is logged in. An exact
    /// selection is carried through untouched, because the caller already owns
    /// the value.
    ///
    /// - `self`: Application service holding the style configuration.
    /// - `context`: Caller context supplying the trusted principal.
    /// - `selection`: Selection carried by the render request.
    ///
    /// # Returns
    /// The resolved selection, which a provider is allowed to render with.
    ///
    /// # Errors
    /// Returns an invalid-request error when a kind label is malformed or a
    /// personal style is named outside the export purpose, an
    /// unsupported-operation error when no domain adapter owns the kind, a schema
    /// violation when the domain rejects a configured style, and store errors when
    /// configuration cannot be read.
    pub fn resolve_render_style(
        &self,
        context: &ApplicationContext,
        selection: &RenderStyleSelection,
    ) -> Result<RenderStyleSelection, ApplicationError> {
        match selection {
            RenderStyleSelection::Applied { .. } => Ok(selection.clone()),
            RenderStyleSelection::Exact { value } => Ok(RenderStyleSelection::Applied {
                value: value.clone(),
                provenance: vec!["caller-exact-style".to_owned()],
            }),
            RenderStyleSelection::Configured {
                projection_kind,
                project_id,
                workspace_id,
                purpose,
                view_id,
                personal_style,
            } => {
                let kind = sr_projection::ProjectionKind::new(projection_kind.clone()).map_err(
                    |error| ApplicationError::InvalidRequest {
                        operation: "render style resolve".to_owned(),
                        message: format!("invalid projection kind: {error}"),
                    },
                )?;
                let mut request = StyleResolutionRequest::new(
                    kind,
                    project_id.clone(),
                    workspace_id.clone(),
                    purpose.projection_purpose(),
                );
                request.view_id.clone_from(view_id);
                request.explicit_personal =
                    personal_style.clone().map(sr_projection::StyleValue::new);
                if matches!(
                    purpose,
                    crate::RenderStylePurpose::InteractiveView
                        | crate::RenderStylePurpose::ExplicitExport
                ) {
                    request.principal = personal_principal(context).map(str::to_owned);
                }
                let resolution = resolve_effective_style(
                    self.projection_style.adapter.as_ref(),
                    self.projection_style.store.as_ref(),
                    &self.projection_style.conventions,
                    &request,
                )?;
                Ok(RenderStyleSelection::Applied {
                    value: resolution.provenance.resolved.value().as_bytes().to_vec(),
                    provenance: resolution
                        .provenance
                        .applied
                        .iter()
                        .map(|source| format!("{:?}", source.kind))
                        .collect(),
                })
            }
        }
    }
}

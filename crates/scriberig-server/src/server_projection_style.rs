//! Adapts projection style wire values to shared application semantics.

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, ProjectionStyleBackend,
    ProjectionStyleSettingRequest as AppStyleSettingRequest, StyleResolutionRequest,
};
use scriberig_protocol as wire;
use sr_projection::{ProjectionKind, ProjectionPurpose, StyleRef, StyleValue};

/// Parses one stable projection kind name.
///
/// - `value`: Kind name supplied by the protocol caller.
///
/// The validated projection kind.
///
/// # Returns
/// The parsed projection kind.
///
/// # Errors
/// Returns an invalid-request error for an empty or malformed kind name.
fn parse_kind(value: &str) -> Result<ProjectionKind, ApplicationError> {
    ProjectionKind::new(value).map_err(|error| ApplicationError::InvalidRequest {
        operation: "projection style".to_owned(),
        message: format!("invalid projection kind: {error}"),
    })
}

/// Converts a shared application style schema to its wire DTO.
///
/// - `schema`: Domain-owned style schema.
///
/// A transport-safe schema with the same ordered property contract.
///
/// # Returns
/// The wire schema response.
pub(super) fn schema_response(
    schema: sr_projection::StyleSchema,
) -> wire::ProjectionStyleSchemaResponse {
    wire::ProjectionStyleSchemaResponse {
        reference: wire::ProjectionStyleSchemaReference {
            adapter_id: schema.reference.adapter_id().to_owned(),
            version: schema.reference.version().to_owned(),
        },
        projection_kinds: schema
            .projection_kinds
            .iter()
            .map(|kind| kind.as_str().to_owned())
            .collect(),
        properties: schema
            .properties
            .into_iter()
            .map(|property| wire::ProjectionStyleProperty {
                key: property.key,
                label: property.label,
                description: property.description,
                kind: match property.kind {
                    sr_projection::StyleValueKind::Boolean => {
                        wire::ProjectionStyleValueKind::Boolean
                    }
                    sr_projection::StyleValueKind::Integer => {
                        wire::ProjectionStyleValueKind::Integer
                    }
                    sr_projection::StyleValueKind::Enum => wire::ProjectionStyleValueKind::Enum,
                    sr_projection::StyleValueKind::Text => wire::ProjectionStyleValueKind::Text,
                    sr_projection::StyleValueKind::Opaque => wire::ProjectionStyleValueKind::Opaque,
                },
                allowed_values: property.allowed_values,
                minimum: property.minimum,
                maximum: property.maximum,
                default: property.default,
                applicability: match property.applicability {
                    sr_projection::StyleApplicability::ViewOnly => {
                        wire::ProjectionStyleApplicability::ViewOnly
                    }
                    sr_projection::StyleApplicability::MaterializationOnly => {
                        wire::ProjectionStyleApplicability::MaterializationOnly
                    }
                    sr_projection::StyleApplicability::Both => {
                        wire::ProjectionStyleApplicability::Both
                    }
                },
                affects_fidelity: property.affects_fidelity,
                required_for_materialization: property.required_for_materialization,
            })
            .collect(),
    }
}

/// Converts an exact style and its provenance to its wire DTO.
///
/// - `style`: Effective application style, ordered sources, and fidelity.
///
/// The complete transport-safe resolution response.
///
/// # Returns
/// The wire response preserving exact bytes and fidelity.
fn resolve_response(
    style: scriberig_app::EffectiveProjectionStyle,
) -> wire::ProjectionStyleResolveResponse {
    let reference = match style.resolved.reference() {
        StyleRef::Value(_) => wire::ProjectionStyleReference::Inline,
        StyleRef::Revision { entity, revision } => wire::ProjectionStyleReference::Revision {
            entity: entity.to_string(),
            revision: revision.to_string(),
        },
    };
    wire::ProjectionStyleResolveResponse {
        purpose: purpose_to_wire(style.purpose),
        reference,
        value: style.resolved.value().as_bytes().to_vec(),
        schema: style
            .resolved
            .schema()
            .map(|schema| wire::ProjectionStyleSchemaReference {
                adapter_id: schema.adapter_id().to_owned(),
                version: schema.version().to_owned(),
            }),
        applied: style
            .applied
            .into_iter()
            .map(|source| wire::ProjectionStyleSource {
                kind: source.kind.to_string(),
                identity: source.identity,
                revision: source.revision.map(|revision| revision.to_string()),
                generation: source.generation,
                value: source.value.as_bytes().to_vec(),
            })
            .collect(),
        fidelity: wire::ProjectionStyleFidelityReport {
            fidelity: match style.fidelity.fidelity {
                sr_projection::ProjectionFidelity::LosslessEditable => {
                    wire::ProjectionStyleFidelity::LosslessEditable
                }
                sr_projection::ProjectionFidelity::LossAwareEditable => {
                    wire::ProjectionStyleFidelity::LossAwareEditable
                }
                sr_projection::ProjectionFidelity::ReadOnlyLossy => {
                    wire::ProjectionStyleFidelity::ReadOnlyLossy
                }
            },
            omitted_categories: style.fidelity.omitted_categories,
            explanation: style.fidelity.explanation,
        },
    }
}

/// Converts a wire purpose to the shared application purpose.
///
/// - `purpose`: Transport purpose selector.
///
/// The shared application purpose.
///
/// # Returns
/// The corresponding domain-neutral purpose.
fn purpose_from_wire(purpose: wire::ProjectionStylePurpose) -> ProjectionPurpose {
    match purpose {
        wire::ProjectionStylePurpose::InteractiveView => ProjectionPurpose::InteractiveView,
        wire::ProjectionStylePurpose::SharedMaterialization => {
            ProjectionPurpose::SharedMaterialization
        }
        wire::ProjectionStylePurpose::ExplicitExport => ProjectionPurpose::ExplicitExport,
    }
}

/// Converts a shared application purpose to its wire representation.
///
/// - `purpose`: Shared application purpose.
///
/// The transport purpose selector.
///
/// # Returns
/// The corresponding wire value.
fn purpose_to_wire(purpose: ProjectionPurpose) -> wire::ProjectionStylePurpose {
    match purpose {
        ProjectionPurpose::InteractiveView => wire::ProjectionStylePurpose::InteractiveView,
        ProjectionPurpose::SharedMaterialization => {
            wire::ProjectionStylePurpose::SharedMaterialization
        }
        ProjectionPurpose::ExplicitExport => wire::ProjectionStylePurpose::ExplicitExport,
    }
}

/// Resolves an effective style from a typed protocol request.
///
/// - `service`: Shared application service owning the style resolver.
/// - `context`: Trusted principal and request context.
/// - `request`: Purpose and scope fields from the protocol caller.
///
/// The exact style, applied provenance, and fidelity.
///
/// # Returns
/// The style resolution response.
///
/// # Errors
/// Returns application validation, configuration, or storage errors.
pub(super) fn resolve<B: scriberig_app::ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &wire::ProjectionStyleResolveRequest,
) -> Result<wire::ProjectionStyleResolveResponse, ApplicationError> {
    let mut app_request = StyleResolutionRequest::new(
        parse_kind(&request.projection_kind)?,
        &request.project_id,
        &request.workspace_id,
        purpose_from_wire(request.purpose),
    );
    app_request.principal = context
        .principal
        .as_ref()
        .map(|principal| principal.id.clone());
    app_request.view_id.clone_from(&request.view_id);
    app_request.explicit_personal = request.explicit_personal.clone().map(StyleValue::new);
    let style = ProjectionStyleBackend::projection_style_effective(service, context, &app_request)?;
    Ok(resolve_response(style))
}

/// Returns one adapter-owned schema through the shared application service.
///
/// - `service`: Shared application service owning the style adapter.
/// - `context`: Trusted request context.
/// - `projection_kind`: Stable projection kind name.
///
/// The schema when the domain adapter declares one.
///
/// # Returns
/// The optional wire schema.
///
/// # Errors
/// Returns application errors from style discovery.
pub(super) fn schema<B: scriberig_app::ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    projection_kind: &str,
) -> Result<Option<wire::ProjectionStyleSchemaResponse>, ApplicationError> {
    let request = scriberig_app::ProjectionStyleSchemaRequest {
        projection_kind: parse_kind(projection_kind)?,
    };
    ProjectionStyleBackend::projection_style_schema(service, context, &request)
        .map(|schema| schema.map(schema_response))
}

/// Sets or clears one style configuration through the shared application service.
///
/// - `service`: Shared application service owning style persistence.
/// - `context`: Trusted principal and authorization context.
/// - `request`: Explicit wire scope, kind, revision, and optional bytes.
///
/// The authoritative stored value and revision.
///
/// # Returns
/// The style mutation response.
///
/// # Errors
/// Returns authorization, revision conflict, validation, or storage errors.
pub(super) fn set<B: scriberig_app::ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &wire::ProjectionStyleSettingRequest,
) -> Result<wire::ProjectionStyleSettingResponse, ApplicationError> {
    let projection_kind = parse_kind(&request.projection_kind)?;
    let style = request.style.clone().map(StyleValue::new);
    let setting = |scope_id: String| AppStyleSettingRequest {
        projection_kind: projection_kind.clone(),
        scope_id,
        expected_revision: request.expected_revision,
        style: style.clone(),
    };
    let stored = match &request.scope {
        wire::ProjectionStyleScope::User => {
            ProjectionStyleBackend::set_projection_style_user_preference(
                service,
                context,
                &setting(String::new()),
            )?
        }
        wire::ProjectionStyleScope::View { view_id } => {
            ProjectionStyleBackend::set_projection_style_view_override(
                service,
                context,
                &setting(view_id.clone()),
            )?
        }
        wire::ProjectionStyleScope::Project { project_id } => {
            ProjectionStyleBackend::set_projection_style_project_policy(
                service,
                context,
                &setting(project_id.clone()),
            )?
        }
        wire::ProjectionStyleScope::Workspace { workspace_id } => {
            ProjectionStyleBackend::set_projection_style_materialization_policy(
                service,
                context,
                &setting(workspace_id.clone()),
            )?
        }
    };
    Ok(wire::ProjectionStyleSettingResponse {
        style: stored
            .as_ref()
            .map(|stored| stored.value.as_bytes().to_vec()),
        revision: stored.map(|stored| stored.revision),
    })
}

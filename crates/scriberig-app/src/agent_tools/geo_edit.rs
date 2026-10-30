//! Provides candidate-only geospatial attribute edits to agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_geo::GeoValue;
use scriberig_project::GeoPackageFeatureAttributeEdit;

/// Stable semantic tool identifier for geospatial attribute changes.
pub const GEO_ATTRIBUTE_EDIT_TOOL: &str = "geo.set_attribute";

/// Stable semantic tool identifier for geospatial geometry changes.
pub const GEO_GEOMETRY_EDIT_TOOL: &str = "geo.replace_geometry";

/// Creates exact-snapshot geospatial attribute candidates through shared operations.
pub struct GeoAttributeEditTool<B> {
    /// Shared application service enforcing snapshot and project authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> GeoAttributeEditTool<B> {
    /// Creates a geospatial edit tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only tool for semantic geospatial attribute edits.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for GeoAttributeEditTool<B> {
    /// Returns the stable geospatial attribute edit tool name.
    ///
    /// # Returns
    /// The registered semantic tool identifier.
    fn name(&self) -> &'static str {
        GEO_ATTRIBUTE_EDIT_TOOL
    }

    /// Rejects generic input because edits require an explicit semantic target.
    ///
    /// # Parameters
    /// - `_context`: Trusted application request context.
    /// - `_input`: Generic tool input without typed feature arguments.
    ///
    /// # Returns
    /// Always returns a structured invalid-input failure.
    ///
    /// # Errors
    /// Directs the caller to the named `geo.set_attribute` contract.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "geospatial edits require named source, layer, feature, field, and value arguments",
        ))
    }

    /// Creates a candidate for one exact semantic feature field.
    ///
    /// # Parameters
    /// - `context`: Trusted exact-snapshot context with a scoped Propose grant.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with path, layer, feature, field, and typed value.
    ///
    /// # Returns
    /// Candidate handle for shared review, validation, and apply.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, stale-base, or candidate errors.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != GEO_ATTRIBUTE_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown geospatial tool".to_owned(),
            }));
        }
        let snapshot = context.snapshot.ok_or_else(|| {
            invalid_argument("geospatial edits require an exact snapshot context")
        })?;
        let path = string_argument(arguments, "path")?;
        let layer = entity_argument(arguments, "layer")?;
        let feature = entity_argument(arguments, "feature")?;
        let field = string_argument(arguments, "field")?;
        let value = serde_json::from_value::<GeoValue>(
            arguments
                .get("value")
                .cloned()
                .ok_or_else(|| invalid_argument("value must be a typed GeoValue"))?,
        )
        .map_err(|_| invalid_argument("value must be a typed GeoValue"))?;
        let candidate = self
            .service
            .propose_geo_feature_attribute(
                context,
                GeoPackageFeatureAttributeEdit {
                    project: self.project,
                    base_snapshot: snapshot,
                    path: path.to_owned(),
                    layer,
                    feature,
                    field: field.to_owned(),
                    value,
                },
            )
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: feature,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("geo.candidate".to_owned()),
            summary: Some(format!(
                "Candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a geospatial edit before candidate creation when the branch is cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application request context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Provider-supplied typed argument object.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review.
    ///
    /// # Errors
    /// Returns cancellation or the named operation's structured failure.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ToolError::Application {
                category: "operation_cancelled".to_owned(),
                message: "geospatial edit was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Creates exact-snapshot geospatial geometry candidates through shared operations.
pub struct GeoGeometryEditTool<B> {
    /// Shared application service enforcing snapshot and project authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> GeoGeometryEditTool<B> {
    /// Creates a geospatial geometry tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only tool for semantic geospatial geometry edits.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for GeoGeometryEditTool<B> {
    /// Returns the stable geospatial geometry edit tool name.
    ///
    /// # Returns
    /// The registered semantic tool identifier.
    fn name(&self) -> &'static str {
        GEO_GEOMETRY_EDIT_TOOL
    }

    /// Rejects generic input because geometry edits require explicit typed arguments.
    ///
    /// # Parameters
    /// - `_context`: Trusted application request context.
    /// - `_input`: Generic tool input without typed geometry arguments.
    ///
    /// # Returns
    /// Always returns an invalid-input failure directing callers to the named contract.
    ///
    /// # Errors
    /// Returns the named `geo.replace_geometry` argument requirement.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "geospatial geometry edits require path, layer, feature, and typed geometry arguments",
        ))
    }

    /// Creates a candidate for one exact semantic feature geometry.
    ///
    /// # Parameters
    /// - `context`: Trusted exact-snapshot context with a scoped Propose grant.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with path, layer, feature, and typed geometry.
    ///
    /// # Returns
    /// Candidate handle for shared review, validation, and apply.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, stale-base, or candidate errors.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != GEO_GEOMETRY_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown geospatial geometry tool".to_owned(),
            }));
        }
        let path = string_argument(arguments, "path")?;
        let layer = entity_argument(arguments, "layer")?;
        let feature = entity_argument(arguments, "feature")?;
        let geometry = serde_json::from_value::<scriberig_domain_geo::Geometry>(
            arguments
                .get("geometry")
                .cloned()
                .ok_or_else(|| invalid_argument("geometry must be a typed Geometry"))?,
        )
        .map_err(|_| invalid_argument("geometry must be a typed Geometry"))?;
        let candidate = self
            .service
            .propose_geo_feature_geometry(context, self.project, path, layer, feature, &geometry)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: feature,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("geo.candidate".to_owned()),
            summary: Some(format!(
                "Candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a geometry edit before candidate creation when the branch is cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application request context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Provider-supplied typed argument object.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review.
    ///
    /// # Errors
    /// Returns cancellation or the named operation's structured failure.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ToolError::Application {
                category: "operation_cancelled".to_owned(),
                message: "geospatial geometry edit was cancelled before candidate creation"
                    .to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Reads one required string argument.
///
/// # Parameters
/// - `arguments`: Named tool argument object.
/// - `name`: Required field name.
///
/// # Returns
/// The exact authored string value.
///
/// # Errors
/// Returns invalid-input when the value is absent or not a string.
fn string_argument<'a>(arguments: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    arguments
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument(&format!("{name} must be a string")))
}

/// Parses one required semantic entity identifier.
///
/// # Parameters
/// - `arguments`: Named tool argument object.
/// - `name`: Required entity identifier field.
///
/// # Returns
/// Stable Scriberig entity identity.
///
/// # Errors
/// Returns invalid-input when the identifier is absent or malformed.
fn entity_argument(arguments: &serde_json::Value, name: &str) -> Result<EntityId, ToolError> {
    string_argument(arguments, name)?
        .parse::<EntityId>()
        .map_err(|_| invalid_argument(&format!("{name} must be a valid entity ID")))
}

/// Creates an invalid geospatial tool argument error.
///
/// # Parameters
/// - `message`: Safe argument diagnostic.
///
/// # Returns
/// Structured invalid-input tool error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts application failures while retaining capability denials.
///
/// # Parameters
/// - `error`: Structured application failure.
///
/// # Returns
/// Provider-neutral tool failure.
fn tool_error(error: ApplicationError) -> ToolError {
    match error {
        ApplicationError::PermissionDenied { action, scope } => {
            ToolError::CapabilityDenied { action, scope }
        }
        other => ToolError::Application {
            category: other.category().to_owned(),
            message: other.to_string(),
        },
    }
}

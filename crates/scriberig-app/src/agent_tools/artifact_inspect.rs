//! Provides bounded, read-only artifact summaries to semantic agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};

/// Stable identifier for bounded artifact catalog reads.
pub const ARTIFACT_INSPECT_TOOL: &str = "artifact.inspect";

/// Reads bounded artifact summaries through the shared application service.
pub struct ArtifactInspectTool<'a, B> {
    /// Shared service enforcing project Inspect capabilities.
    service: &'a ApplicationService<B>,
    /// Exact project scope for this tool instance.
    project: ProjectId,
}

impl<'a, B> ArtifactInspectTool<'a, B> {
    /// Creates an artifact reader bound to one project.
    ///
    /// - `service`: Shared application service used for authorized reads.
    /// - `project`: Exact project scope for this reader.
    ///
    /// The bounded artifact reader.
    #[must_use]
    pub const fn new(service: &'a ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for ArtifactInspectTool<'_, B> {
    /// Returns the stable artifact inspection tool name.
    ///
    /// # Returns
    /// The provider-facing tool identifier.
    fn name(&self) -> &'static str {
        ARTIFACT_INSPECT_TOOL
    }

    /// Reads the first bounded artifact page through application authorization.
    ///
    /// - `context`: Trusted request context carrying project Inspect grants.
    /// - `_input`: Provider-neutral input; paging is exposed by named arguments.
    ///
    /// A bounded summary result with stable artifact handles.
    ///
    /// # Returns
    /// The first page of artifact summaries.
    ///
    /// # Errors
    /// Returns a capability denial or an application error.
    fn invoke(
        &self,
        context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        self.read_page(context, 0, 20)
    }

    /// Reads a caller-selected bounded artifact page.
    ///
    /// - `context`: Trusted request context carrying project Inspect grants.
    /// - `name`: Requested registered tool name.
    /// - `arguments`: Optional `offset` and `limit` page bounds.
    ///
    /// A bounded summary result with stable artifact handles.
    ///
    /// # Returns
    /// The requested artifact page.
    ///
    /// # Errors
    /// Returns an invalid request, capability denial, or application error.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != ARTIFACT_INSPECT_TOOL {
            return Err(ToolError::Application {
                category: "unsupported_operation".to_owned(),
                message: format!("unknown artifact tool: {name}"),
            });
        }
        let offset = optional_bound(arguments, "offset", 0)?;
        let limit = optional_bound(arguments, "limit", 20)?;
        self.read_page(context, offset, limit)
    }
}

impl<B> ArtifactInspectTool<'_, B> {
    /// Reads and serializes one bounded page without including source bytes.
    ///
    /// - `self`: Reader bound to an exact project.
    /// - `context`: Trusted request context with the required Inspect grant.
    /// - `offset`: Zero-based artifact page offset.
    /// - `limit`: Maximum number of summaries requested.
    ///
    /// A summary result containing stable artifact handles and manifest metadata.
    ///
    /// # Returns
    /// The bounded artifact result.
    ///
    /// # Errors
    /// Returns a capability denial, application error, or serialization error.
    fn read_page(
        &self,
        context: &ApplicationContext,
        offset: usize,
        limit: usize,
    ) -> Result<ToolResult, ToolError> {
        let page = self
            .service
            .inspect_artifacts(context, self.project, offset, limit)
            .map_err(map_application_error)?;
        let handles = page
            .artifacts
            .iter()
            .map(|artifact| artifact.id.entity().to_string())
            .collect();
        let entity = page.artifacts.first().map_or_else(
            || EntityId::from_bytes(*self.project.as_bytes()),
            |item| item.id.entity(),
        );
        let summary = serde_json::to_string(&serde_json::json!({
            "project": page.project,
            "total": page.total,
            "next_offset": page.next_offset,
            "artifacts": page.artifacts,
        }))
        .map_err(|error| ToolError::Application {
            category: "serialization_failed".to_owned(),
            message: error.to_string(),
        })?;
        Ok(ToolResult {
            entity,
            revision: None,
            snapshot: context.snapshot,
            object_type: Some("artifact.page".to_owned()),
            summary: Some(summary),
            handles,
        })
    }
}

/// Parses one optional non-negative page bound.
///
/// - `arguments`: Provider-supplied argument object.
/// - `key`: Bound field name.
/// - `default`: Value used when the field is omitted.
///
/// The validated page bound.
///
/// # Returns
/// The supplied bound or its default.
///
/// # Errors
/// Returns an invalid request when the field is not a non-negative integer.
fn optional_bound(
    arguments: &serde_json::Value,
    key: &str,
    default: usize,
) -> Result<usize, ToolError> {
    arguments.get(key).map_or(Ok(default), |value| {
        value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| ToolError::Application {
                category: "invalid_request".to_owned(),
                message: format!("artifact page {key} must be a non-negative integer"),
            })
    })
}

/// Maps shared application failures into the provider-neutral tool error.
///
/// - `error`: Failure returned by the shared artifact operation.
///
/// The corresponding agent tool error.
///
/// # Returns
/// A capability denial or categorized application error.
fn map_application_error(error: ApplicationError) -> ToolError {
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

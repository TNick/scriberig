//! Parses and authorizes provider-facing agent control requests.

use scriberig_agent_core::ToolError;
use scriberig_app::{ApplicationContext, CapabilityAction, CapabilityId, CapabilityScope};
use scriberig_core::{EntityId, OperationId};

/// Checks a named control capability in the trusted request context.
///
/// - `super`: Module namespace containing control request helpers.
/// - `context`: Trusted application authorization context.
/// - `name`: Capability identifier required by the tool.
///
/// Unit when the capability is granted.
///
/// Returns a capability denial or invalid capability failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn require_capability(
    context: &ApplicationContext,
    name: &str,
) -> Result<(), ToolError> {
    let id = CapabilityId::new(name).map_err(|error| ToolError::Application {
        category: "invalid_capability".to_owned(),
        message: error.to_owned(),
    })?;
    if context.capabilities.allows(&id)
        || context.capabilities.permits(
            &CapabilityAction::Custom(name.to_owned()),
            &CapabilityScope::Project,
            None,
            None,
        )
    {
        return Ok(());
    }
    Err(ToolError::CapabilityDenied {
        action: name.to_owned(),
        scope: "project".to_owned(),
    })
}

/// Parses one entity identifier argument.
///
/// - `super`: Module namespace containing control request helpers.
/// - `arguments`: Provider argument object.
/// - `name`: Entity argument key.
///
/// Parsed entity identity.
///
/// Returns a structured invalid-request failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn argument_id(
    arguments: &serde_json::Value,
    name: &str,
) -> Result<EntityId, ToolError> {
    arguments
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument(name))?
        .parse::<EntityId>()
        .map_err(|error| {
            let message = error.to_string();
            invalid_value(name, &message)
        })
}

/// Parses one operation identifier argument.
///
/// - `super`: Module namespace containing control request helpers.
/// - `arguments`: Provider argument object.
///
/// Parsed operation identity.
///
/// Returns a structured invalid-request failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn argument_operation(arguments: &serde_json::Value) -> Result<OperationId, ToolError> {
    arguments
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("operation"))?
        .parse::<OperationId>()
        .map_err(|error| {
            let message = error.to_string();
            invalid_value("operation", &message)
        })
}

/// Builds a structured invalid-argument failure.
///
/// - `super`: Module namespace containing control request helpers.
/// - `name`: Missing argument name.
///
/// A structured invalid-request error.
/// # Returns
/// The operation result.
pub(super) fn invalid_argument(name: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_request".to_owned(),
        message: format!("agent control argument is required: {name}"),
    }
}

/// Builds a structured invalid-value failure.
///
/// - `name`: Invalid argument name.
/// - `message`: Safe parser detail.
///
/// A structured invalid-request error.
/// # Returns
/// The operation result.
fn invalid_value(name: &str, message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_request".to_owned(),
        message: format!("invalid {name}: {message}"),
    }
}

/// Maps an application failure into the agent tool boundary.
///
/// - `super`: Module namespace containing control request helpers.
/// - `error`: Application failure to classify.
///
/// Provider-neutral tool failure.
/// # Returns
/// The operation result.
pub(super) fn map_error(error: &scriberig_app::ApplicationError) -> ToolError {
    ToolError::Application {
        category: error.category().to_owned(),
        message: error.to_string(),
    }
}

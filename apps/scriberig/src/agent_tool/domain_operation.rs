//! Executes installed semantic mutations through the shared application API.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationRequest, ApplicationResult, BranchContext,
    CapabilityId, DomainOperationRequest,
};
use scriberig_core::{EntityId, RevisionId, RootName, SnapshotId};
use scriberig_query::{ProjectionAnchor, QueryValue, RelationStep, Selection};

use super::LocalAgentTool;
use crate::agent_tool_helpers::{invalid_argument, map_error, require_capability};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Invokes one installed semantic capability against an exact entity.
    ///
    /// - `context`: Trusted exact-snapshot and semantic-operation grants.
    /// - `arguments`: Capability, entity, snapshot, optional revision, and
    ///   scalar domain arguments.
    /// - `super`: Parent module containing the agent tool adapter.
    ///
    /// Candidate change handle from the installed domain capability.
    ///
    /// Returns scope, schema, capability, or application-operation failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn domain_operation(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        require_capability(context, "domain_operation")?;
        let capability = arguments
            .get("capability")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("capability"))?;
        let capability = CapabilityId::new(capability).map_err(|error| ToolError::Application {
            category: "invalid_argument".to_owned(),
            message: error.to_owned(),
        })?;
        let entity = parse_entity(arguments)?;
        let snapshot = parse_snapshot(arguments)?;
        if context.snapshot != Some(snapshot) {
            return Err(ToolError::CapabilityDenied {
                action: "domain_operation".to_owned(),
                scope: format!("snapshot:{snapshot}"),
            });
        }
        let revision = arguments
            .get("revision")
            .and_then(serde_json::Value::as_str)
            .map(str::parse::<RevisionId>)
            .transpose()
            .map_err(|error| invalid_value("revision", &error.to_string()))?;
        let branch = arguments
            .get("branch")
            .and_then(serde_json::Value::as_str)
            .map(RootName::new)
            .transpose()
            .map_err(|error| invalid_value("branch", &error.to_string()))?;
        let domain_arguments = parse_domain_arguments(arguments)?;
        let request = DomainOperationRequest {
            capability: capability.clone(),
            subject: Selection {
                snapshot,
                entity,
                revision,
                relation_path: Vec::<RelationStep>::new(),
                projection_anchor: None::<ProjectionAnchor>,
            },
            context: BranchContext {
                base_snapshot: snapshot,
                branch,
            },
            arguments: domain_arguments,
        };
        let result = self
            .service
            .execute(context, &ApplicationRequest::DomainOperation(request))
            .map_err(|error| map_error(&error))?;
        let ApplicationResult::DomainOperation(execution) = result else {
            return Err(ToolError::Application {
                category: "invalid_application_result".to_owned(),
                message: "semantic operation returned an incompatible result".to_owned(),
            });
        };
        let mut output = Self::result(
            format!("semantic operation {capability} produced a candidate change"),
            vec![
                format!("semantic_operation:{capability}"),
                format!("candidate_change:{}", execution.change),
                format!("entity:{entity}"),
                format!("snapshot:{snapshot}"),
            ],
        );
        output.entity = entity;
        output.snapshot = Some(snapshot);
        output.object_type = Some("semantic_candidate".to_owned());
        Ok(output)
    }
}

/// Parses the exact semantic subject entity.
///
/// - `arguments`: Provider-supplied JSON object.
///
/// Stable semantic entity identity.
///
/// Returns a tool argument error for a missing or malformed identity.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_entity(arguments: &serde_json::Value) -> Result<EntityId, ToolError> {
    arguments
        .get("entity")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("entity"))?
        .parse::<EntityId>()
        .map_err(|error| invalid_value("entity", &error.to_string()))
}

/// Parses the exact immutable base snapshot.
///
/// - `arguments`: Provider-supplied JSON object.
///
/// Snapshot identity selected by the caller.
///
/// Returns a tool argument error for a missing or malformed snapshot.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_snapshot(arguments: &serde_json::Value) -> Result<SnapshotId, ToolError> {
    arguments
        .get("snapshot")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument("snapshot"))?
        .parse::<SnapshotId>()
        .map_err(|error| invalid_value("snapshot", &error.to_string()))
}

/// Parses bounded scalar arguments for one domain operation.
///
/// - `arguments`: Provider-supplied JSON object.
///
/// Sorted semantic argument names and values.
///
/// Returns a tool argument error for non-scalar values or reserved keys.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_domain_arguments(
    arguments: &serde_json::Value,
) -> Result<Vec<(String, QueryValue)>, ToolError> {
    let Some(values) = arguments.get("arguments") else {
        return Ok(Vec::new());
    };
    let values = values
        .as_object()
        .ok_or_else(|| invalid_argument("arguments"))?;
    if values.len() > 64 {
        return Err(invalid_value("arguments", "at most 64 values are allowed"));
    }
    values
        .iter()
        .map(|(name, value)| {
            if name.is_empty() || name.len() > 128 {
                return Err(invalid_value("arguments", "argument names must be bounded"));
            }
            let value = if let Some(text) = value.as_str() {
                QueryValue::Text(text.to_owned())
            } else if let Some(integer) = value.as_i64() {
                QueryValue::Integer(integer)
            } else if let Some(boolean) = value.as_bool() {
                QueryValue::Boolean(boolean)
            } else {
                return Err(invalid_value(
                    "arguments",
                    "domain arguments must be strings, integers, or booleans",
                ));
            };
            Ok((name.clone(), value))
        })
        .collect()
}

/// Builds a safe invalid-value tool error.
///
/// - `name`: Argument name whose value could not be parsed.
/// - `message`: Safe parser or validation detail.
///
/// Structured tool input failure.
/// # Returns
/// The operation result.
fn invalid_value(name: &str, message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_argument".to_owned(),
        message: format!("invalid {name}: {message}"),
    }
}

#[cfg(test)]
#[path = "domain_operation_tests.rs"]
mod tests;

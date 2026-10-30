//! Builds upstream Faux provider calls and `ScribeRig` semantic branch tools.

use std::sync::Arc;

use rpi_agent::{AgentError, AgentToolResult};
use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use rpi_ai::types::Tool as ProviderTool;
use scriberig_agent_core::{Tool, ToolInput};
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_app::{ApplicationContext, CapabilityContext};
use scriberig_core::{EntityId, SnapshotId};
use serde_json::{Value, json};

use super::ProjectAgentTool;

/// Creates the deterministic upstream Faux provider and its selected model.
///
/// - `entity`: Selected semantic entity.
/// - `snapshot`: Selected exact snapshot.
/// - `accepted`: Deterministic accepted source text.
///
/// # Returns
/// A deterministic upstream Faux provider and its selected model.
pub(super) fn faux_provider(
    entity: EntityId,
    snapshot: SnapshotId,
    accepted: &str,
) -> (Arc<dyn rpi_ai::Provider>, rpi_ai::Model) {
    let provider = FauxProvider::new(
        FauxScript::new()
            .with_tool_call(
                "inspect",
                json!({"entity": entity.to_string(), "snapshot": snapshot.to_string(), "detail": "detailed"}),
            )
            .with_tool_call("project_propose", json!({"text": accepted}))
            .with_text("Candidate ready for review."),
    );
    let model = provider.default_model().clone();
    (provider, model)
}

/// Creates branch-scoped semantic tools for the project workflow.
///
/// - `tool`: Owned project service adapter.
/// - `branch`: Canonical ephemeral branch identity for this invocation.
/// - `context`: Trusted `ScribeRig` request context.
///
/// # Returns
/// Snapshot-bound inspect and candidate tools.
///
/// # Errors
/// Returns an error if the branch catalog rejects a semantic tool.
pub(super) fn project_tools(
    tool: Arc<ProjectAgentTool>,
    branch: EntityId,
    context: ApplicationContext,
) -> Result<Vec<Arc<SemanticAgentTool>>, String> {
    let inspect = ProviderTool {
        name: "inspect".to_owned(),
        description: "Inspect selected semantic entity.".to_owned(),
        parameters: json!({"type": "object", "required": ["entity", "snapshot"], "properties": {
            "entity": {"type": "string"}, "snapshot": {"type": "string"}, "detail": {"type": "string"}
        }})
        .into(),
        constrained_sampling: None,
    };
    let propose = ProviderTool {
        name: "project_propose".to_owned(),
        description: "Create a reviewed source candidate.".to_owned(),
        parameters: json!({"type": "object", "required": ["text"], "properties": {
            "text": {"type": "string"}
        }})
        .into(),
        constrained_sampling: None,
    };
    let catalog = ToolCatalog::new(vec![
        tool_descriptor(inspect.clone(), false, ToolCategory::SemanticRead),
        tool_descriptor(propose.clone(), true, ToolCategory::SemanticMutation),
    ]);
    let inspect_tool = SemanticAgentTool::new(
        &inspect,
        "Inspect selected source",
        branch,
        catalog.clone(),
        CapabilityContext::default(),
        {
            let tool = Arc::clone(&tool);
            let context = context.clone();
            move |_id, arguments, _signal, _update| {
                let tool = Arc::clone(&tool);
                let context = context.clone();
                async move {
                    let entity = parse_id::<EntityId>(&arguments, "entity")?;
                    let snapshot = parse_id::<SnapshotId>(&arguments, "snapshot")?;
                    let result = tool
                        .invoke(
                            &context,
                            ToolInput {
                                entity,
                                snapshot: Some(snapshot),
                                detail: scriberig_agent_core::ToolDetail::Detailed,
                            },
                        )
                        .map_err(|_| AgentError::tool("semantic inspect failed"))?;
                    Ok(AgentToolResult::text(format!(
                        "{}: {} {}",
                        result.entity,
                        result.summary.unwrap_or_default(),
                        result.handles.join(", ")
                    )))
                }
            }
        },
    )?;
    let propose_tool = SemanticAgentTool::new(
        &propose,
        "Create project candidate",
        branch,
        catalog,
        CapabilityContext::default(),
        move |_id, arguments, _signal, _update| {
            let tool = Arc::clone(&tool);
            let context = context.clone();
            async move {
                let result = tool
                    .invoke_named(&context, "project_propose", &arguments)
                    .map_err(|_| AgentError::tool("candidate proposal failed"))?;
                let id = result
                    .handles
                    .first()
                    .ok_or_else(|| AgentError::tool("candidate identity missing"))?;
                Ok(AgentToolResult::text(format!("candidate created: {id}")))
            }
        },
    )?;
    Ok(vec![Arc::new(inspect_tool), Arc::new(propose_tool)])
}

/// Creates a descriptor retaining `ScribeRig` semantic tool policy.
///
/// - `definition`: Provider-facing schema.
/// - `state_changing`: Whether the operation creates a candidate.
/// - `category`: Semantic operation category.
///
/// # Returns
/// A bounded tool descriptor for the trusted catalog.
fn tool_descriptor(
    definition: ProviderTool,
    state_changing: bool,
    category: ToolCategory,
) -> ToolDescriptor {
    ToolDescriptor {
        definition,
        category,
        result_shape: "bounded semantic result".to_owned(),
        invocation_label: None,
        state_changing,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
    }
}

/// Parses one stable identifier from provider-supplied tool arguments.
///
/// - `arguments`: Provider-supplied JSON object.
/// - `name`: Required string property.
///
/// # Returns
/// The exact parsed semantic identifier.
///
/// # Errors
/// Returns a generic tool error when parsing fails.
fn parse_id<T: std::str::FromStr>(arguments: &Value, name: &str) -> Result<T, AgentError> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| AgentError::tool("invalid semantic tool identifier"))
}

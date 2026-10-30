//! Runs the project candidate flow through the upstream agent loop.

#![allow(clippy::wildcard_imports)]

use super::*;
use rpi_agent::{AgentError, AgentToolResult};
use rpi_ai::types::Tool as ProviderTool;
use scriberig_agent_core::{Tool, ToolDetail, ToolInput};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, ContextCompiler, ContextPolicy, ProjectSemanticScope,
};
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation_blocking,
};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_app::CapabilityContext;
use scriberig_core::EntityId;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[path = "service_operation_agent/faux_provider.rs"]
mod faux_provider_module;
use faux_provider_module::faux_provider;

/// Runs one semantic project agent and stores its normal candidate review state.
///
/// - `adapter`: Application operation adapter holding project workflow state.
/// - `context`: Trusted runtime context for semantic tools.
/// - `entity`: Selected source entity.
/// - `snapshot`: Exact candidate base snapshot.
/// - `prompt`: User-authored change instruction.
///
/// The normal application candidate review summary.
///
/// Returns a structured agent, validation, or storage failure.
pub(super) fn project_agent<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    entity: EntityId,
    snapshot: SnapshotId,
    prompt: &str,
) -> Result<ApplicationResult, ApplicationError> {
    let projection = adapter.project_projection.as_ref().ok_or_else(|| {
        ApplicationError::UnsupportedOperation {
            operation: "project agent".to_owned(),
        }
    })?;
    let (path, bytes) =
        projection
            .sources
            .get(&entity)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: entity.to_string(),
            })?;
    let replacement =
        String::from_utf8_lossy(bytes).replace("left + right", "left.saturating_add(right)");
    let branch = EntityId::new();
    let project_context = BranchExecutionContext::new(
        projection.project.to_string(),
        branch.to_string(),
        "project candidate proposal",
        prompt,
    )
    .with_project_scope(ProjectSemanticScope {
        project: projection.project.to_string(),
        snapshot: snapshot.to_string(),
        root: projection.project.to_string(),
        selection: Some(entity.to_string()),
    });
    let compiled_context = ContextCompiler::new(ContextPolicy::default()).compile(&project_context);
    let (provider, model) = faux_provider(entity, snapshot, &replacement);
    let candidate_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let tools = project_tools(
        projection.clone(),
        entity,
        snapshot,
        branch,
        Arc::clone(&adapter.project_candidates),
        Arc::clone(&candidate_id),
        context.clone(),
    )?;
    let invocation = UpstreamBranchInvocation {
        branch,
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some("scriberig-deterministic-project-agent".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context: compiled_context,
        tools,
    };
    run_upstream_branch_invocation_blocking(invocation, prompt).map_err(|error| {
        ApplicationError::OperationFailed {
            operation: "project_agent".to_owned(),
            message: error.error.to_string(),
        }
    })?;
    let candidate_id = candidate_id
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "project candidate identity lock poisoned".to_owned(),
        })?
        .clone()
        .ok_or_else(|| ApplicationError::OperationFailed {
            operation: "project_agent".to_owned(),
            message: "agent did not create a candidate".to_owned(),
        })?;
    let candidate_value = projection
        .workflow
        .candidate(&candidate_id)
        .map_err(|error| ApplicationError::ValidationFailure {
            message: error.to_string(),
        })?
        .ok_or_else(|| ApplicationError::Internal {
            message: "created project candidate disappeared".to_owned(),
        })?;
    let validation = match &candidate_value.validation {
        scriberig_project::CandidateValidation::Pending => scriberig_app::ValidationStatus::Skipped,
        scriberig_project::CandidateValidation::Passed => scriberig_app::ValidationStatus::Passed,
        scriberig_project::CandidateValidation::Failed { .. } => {
            scriberig_app::ValidationStatus::Failed
        }
    };
    Ok(ApplicationResult::Apply(scriberig_app::OperationSummary {
        handle: candidate_value.id,
        snapshot: Some(snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: None,
        validation: Some(validation),
        validation_profile: None,
        validation_diagnostics: Vec::new(),
        review_details: vec![format!(
            "scope: entity={entity} snapshot={snapshot}; {path}: add | old: left + right | new: left.saturating_add(right)"
        )],
        conflicts: Vec::new(),
    }))
}

/// Creates branch-scoped inspect and candidate tools for the project flow.
///
/// - `projection`: Snapshot-bound project source projection.
/// - `entity`: Selected source entity.
/// - `snapshot`: Exact candidate base snapshot.
/// - `branch`: Canonical branch identity for this execution.
/// - `candidate_store`: TUI-owned candidate review cache.
/// - `candidate_id`: Sink receiving the created candidate identity.
/// - `context`: Trusted `ScribeRig` application request context.
///
/// The upstream semantic tools authorized for this branch.
///
/// Returns a safe application error when tool schemas cannot be bound.
/// # Returns
/// Branch-scoped inspect and candidate tools.
///
/// # Errors
/// Returns an application error when schemas cannot be bound.
fn project_tools(
    projection: ProjectProjection,
    entity: EntityId,
    snapshot: SnapshotId,
    branch: EntityId,
    candidate_store: Arc<
        Mutex<std::collections::HashMap<String, scriberig_project::ProjectCandidate>>,
    >,
    candidate_id: Arc<Mutex<Option<String>>>,
    context: ApplicationContext,
) -> Result<Vec<Arc<SemanticAgentTool>>, ApplicationError> {
    let inspect = definition(
        "inspect",
        "Inspect the selected semantic entity.",
        json!({"type": "object", "required": ["entity", "snapshot"], "properties": {
            "entity": {"type": "string"}, "snapshot": {"type": "string"},
            "detail": {"type": "string"}
        }}),
    );
    let propose = definition(
        "project_propose",
        "Create a reviewed source candidate.",
        json!({"type": "object", "required": ["text"], "properties": {
            "text": {"type": "string"}
        }}),
    );
    let catalog = ToolCatalog::new(vec![
        descriptor(inspect.clone(), ToolCategory::SemanticRead, false),
        descriptor(propose.clone(), ToolCategory::SemanticMutation, true),
    ]);
    let tool = Arc::new(ProjectAgentTool {
        projection,
        entity,
        snapshot,
    });
    let inspect_tool = inspect_tool(&inspect, branch, &catalog, &tool, &context)?;
    let propose_tool = candidate_tool(
        &propose,
        branch,
        catalog,
        &tool,
        candidate_store,
        candidate_id,
        context,
    )?;
    Ok(vec![Arc::new(inspect_tool), Arc::new(propose_tool)])
}

/// Creates the snapshot-bound inspect operation for a project branch.
///
/// - `definition`: Provider-visible inspect schema.
/// - `branch`: Canonical branch identity.
/// - `catalog`: Trusted branch tool catalog.
/// - `tool`: Project-backed inspect adapter.
/// - `context`: Trusted application request context.
///
/// The validated inspect tool.
///
/// # Returns
/// An inspect tool bound to the exact branch and snapshot.
///
/// # Errors
/// Returns an application error when the inspect tool cannot be bound.
fn inspect_tool(
    definition: &ProviderTool,
    branch: EntityId,
    catalog: &ToolCatalog,
    tool: &Arc<ProjectAgentTool>,
    context: &ApplicationContext,
) -> Result<SemanticAgentTool, ApplicationError> {
    SemanticAgentTool::new(
        definition,
        "Inspect selected source",
        branch,
        catalog.clone(),
        CapabilityContext::default(),
        {
            let tool = Arc::clone(tool);
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
                                detail: ToolDetail::Detailed,
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
    )
    .map_err(|_| ApplicationError::Internal {
        message: "project inspect tool is not visible to the branch".to_owned(),
    })
}

/// Creates the candidate-writing operation and records its review identity.
///
/// - `definition`: Provider-visible proposal schema.
/// - `branch`: Canonical branch identity.
/// - `catalog`: Trusted branch tool catalog.
/// - `tool`: Project-backed proposal adapter.
/// - `candidate_store`: TUI cache for normal candidate review.
/// - `candidate_id`: Sink receiving the created candidate identity.
/// - `context`: Trusted application request context.
///
/// The validated candidate proposal tool.
///
/// # Returns
/// A candidate tool bound to the exact branch and review cache.
///
/// # Errors
/// Returns an application error when the proposal tool cannot be bound.
fn candidate_tool(
    definition: &ProviderTool,
    branch: EntityId,
    catalog: ToolCatalog,
    tool: &Arc<ProjectAgentTool>,
    candidate_store: Arc<
        Mutex<std::collections::HashMap<String, scriberig_project::ProjectCandidate>>,
    >,
    candidate_id: Arc<Mutex<Option<String>>>,
    context: ApplicationContext,
) -> Result<SemanticAgentTool, ApplicationError> {
    SemanticAgentTool::new(
        definition,
        "Create project candidate",
        branch,
        catalog,
        CapabilityContext::default(),
        {
            let tool = Arc::clone(tool);
            move |_id, arguments, _signal, _update| {
                let tool = Arc::clone(&tool);
                let candidate_store = Arc::clone(&candidate_store);
                let candidate_id = Arc::clone(&candidate_id);
                let context = context.clone();
                async move {
                    let result = tool
                        .invoke_named(&context, "project_propose", &arguments)
                        .map_err(|_| AgentError::tool("candidate proposal failed"))?;
                    let id = result.handles.first().cloned().ok_or_else(|| {
                        AgentError::tool("candidate operation returned no handle")
                    })?;
                    let candidate = tool
                        .projection
                        .workflow
                        .candidate(&id)
                        .map_err(|_| AgentError::tool("candidate lookup failed"))?
                        .ok_or_else(|| AgentError::tool("candidate disappeared"))?;
                    candidate_store
                        .lock()
                        .map_err(|_| AgentError::tool("candidate cache unavailable"))?
                        .insert(id.clone(), candidate);
                    candidate_id
                        .lock()
                        .map_err(|_| AgentError::tool("candidate result unavailable"))?
                        .replace(id.clone());
                    Ok(AgentToolResult::text(format!("candidate created: {id}")))
                }
            }
        },
    )
    .map_err(|_| ApplicationError::Internal {
        message: "project candidate tool is not visible to the branch".to_owned(),
    })
}

/// Creates a stable schema descriptor for the deterministic project tools.
///
/// - `name`: Stable provider-visible operation name.
/// - `description`: Human-readable operation purpose.
/// - `parameters`: JSON schema validated by the upstream agent.
///
/// The trusted provider-facing tool definition.
/// # Returns
/// A provider schema for one trusted semantic operation.
fn definition(name: &str, description: &str, parameters: Value) -> ProviderTool {
    ProviderTool {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters: parameters.into(),
        constrained_sampling: None,
    }
}

/// Adds one trusted operation definition to the branch catalog.
///
/// - `definition`: Provider-facing operation schema.
/// - `category`: Semantic operation category.
/// - `state_changing`: Whether the operation writes a candidate.
///
/// The full branch-visible tool descriptor.
/// # Returns
/// A trusted descriptor with `ScribeRig` semantic category and policy.
fn descriptor(
    definition: ProviderTool,
    category: ToolCategory,
    state_changing: bool,
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

/// Parses one provider argument as a stable semantic identifier.
///
/// - `arguments`: Provider-supplied JSON object.
/// - `name`: Required string field.
///
/// The parsed identifier.
///
/// # Errors
/// Returns an upstream safe tool error when the value is invalid.
fn parse_id<T: std::str::FromStr>(arguments: &Value, name: &str) -> Result<T, AgentError> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| AgentError::tool("invalid semantic tool identifier"))
}

//! Runs the project candidate flow through the upstream agent loop.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_agent_runtime::context::{
    BranchExecutionContext, CompiledContext, ContextCompiler, ContextPolicy, ProjectSemanticScope,
};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation_blocking,
};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_core::EntityId;
use std::sync::{Arc, Mutex};

#[path = "service_operation_agent/faux_provider.rs"]
mod faux_provider_module;
use faux_provider_module::faux_provider;

#[path = "service_operation_agent/project_tools.rs"]
mod project_tools_module;
use project_tools_module::project_tools;

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
    let invocation = project_invocation(branch, provider, model, compiled_context, tools);
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

/// Builds the deterministic upstream invocation for one project candidate branch.
///
/// - `branch`: Canonical `ScribeRig` branch identity for this execution.
/// - `provider`: Deterministic scripted provider for the branch.
/// - `model`: Exact upstream model row the provider serves.
/// - `context`: `ScribeRig`-compiled branch context.
/// - `tools`: Branch-authorized semantic tools only.
///
/// # Returns
/// The upstream branch invocation, carrying its own branch activity sequence.
fn project_invocation(
    branch: EntityId,
    provider: Arc<dyn rpi_ai::Provider>,
    model: rpi_ai::Model,
    context: CompiledContext,
    tools: Vec<Arc<SemanticAgentTool>>,
) -> UpstreamBranchInvocation {
    UpstreamBranchInvocation {
        branch,
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some("scriberig-deterministic-project-agent".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context,
        tools,
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    }
}

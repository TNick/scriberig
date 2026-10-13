//! Runs semantic project-agent requests through the normal candidate path.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_agent_runtime::AgentRuntime;

/// Runs one semantic project agent and stores its normal candidate review state.
///
/// # Parameters
/// - `adapter`: Application operation adapter holding project workflow state.
/// - `context`: Trusted runtime context for semantic tools.
/// - `entity`: Selected source entity.
/// - `snapshot`: Exact candidate base snapshot.
/// - `prompt`: User-authored change instruction.
///
/// # Returns
/// The normal application candidate review summary.
///
/// # Errors
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
    let tool = ProjectAgentTool {
        projection,
        entity,
        snapshot,
    };
    let mut runtime = AgentRuntime::new(golden_provider(entity, snapshot), tool, 4);
    let result = runtime
        .run(context, prompt, &std::sync::atomic::AtomicBool::new(false))
        .map_err(|error| ApplicationError::OperationFailed {
            operation: "project_agent".to_owned(),
            message: error.to_string(),
        })?;
    let candidate = result
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            scriberig_agent_runtime::AgentMessage::Tool(value)
                if value.object_type.as_deref() == Some("migration_candidate") =>
            {
                value.handles.first().cloned()
            }
            _ => None,
        })
        .ok_or_else(|| ApplicationError::OperationFailed {
            operation: "project_agent".to_owned(),
            message: "agent did not create a candidate".to_owned(),
        })?;
    let candidate_value = projection
        .workflow
        .propose_candidate(projection.project, entity, replacement)
        .map_err(|error| ApplicationError::ValidationFailure {
            message: error.to_string(),
        })?;
    if candidate != candidate_value.id {
        return Err(ApplicationError::Internal {
            message: "agent candidate identity changed between tool calls".to_owned(),
        });
    }
    adapter
        .project_candidates
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "project candidate lock poisoned".to_owned(),
        })?
        .insert(candidate_value.id.clone(), candidate_value.clone());
    let validation = match &candidate_value.validation {
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
        validation_diagnostics: Vec::new(),
        conflicts: vec![format!(
            "scope: entity={entity} snapshot={snapshot}; {path}: add | old: left + right | new: left.saturating_add(right)"
        )],
    }))
}

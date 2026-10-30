//! Compiles exact durable conversation state for integrated agent invocations.

use scriberig_agent_runtime::context::{
    BranchExecutionContext, BranchRole, CompiledContext, ContextActivity, ContextCompiler,
    ContextPolicy,
};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{Branch, ContinuationAnchor};

/// Compiles exact persisted branch context for an integrated model invocation.
///
/// The current human instruction is carried by the runtime request itself, so
/// this projection removes it from history to avoid sending it twice.
///
/// - `service`: Shared application service owning the durable session.
/// - `context`: Trusted caller authorization and semantic scope.
/// - `session_id`: Exact session receiving the current turn.
/// - `branch_id`: Exact executing branch identity.
/// - `prompt`: Current human instruction, passed separately to the runtime.
/// - `project_scope`: Optional exact semantic workspace selection.
///
/// Bounded provider-neutral context for the selected continuation branch.
///
/// Returns a provider-runtime failure when session or anchor state is invalid.
pub(crate) fn compile_branch_context<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session_id: EntityId,
    branch_id: EntityId,
    prompt: &str,
    project_scope: Option<scriberig_agent_runtime::context::ProjectSemanticScope>,
) -> Result<CompiledContext, scriberig_agent_runtime::AgentError> {
    let session = SessionBackend::read_session(
        service,
        context,
        &SessionRequest {
            session: session_id,
        },
    )
    .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))?;
    let branch = session
        .branches
        .iter()
        .find(|branch| branch.id == branch_id)
        .ok_or_else(|| {
            scriberig_agent_runtime::AgentError::Provider(
                "executing branch is absent from durable session".to_owned(),
            )
        })?;
    let anchor = branch.continuation.map_or(
        ContinuationAnchor::BranchHead { branch: branch_id },
        |provenance| provenance.anchor,
    );
    let mut execution = BranchExecutionContext::from_session_anchor(
        &session,
        branch_id,
        anchor,
        "semantic agent continuation",
        prompt,
    )
    .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))?;
    let branch_role = branch
        .parent
        .map_or(BranchRole::Root, |parent| BranchRole::NonRoot {
            parent: parent.to_string(),
        });
    execution = execution.with_branch_role(branch_role);
    if let Some(scope) = project_scope {
        execution = execution.with_project_scope(scope);
    }
    add_branch_history(
        &mut execution,
        branch,
        prompt,
        branch.continuation.is_some(),
    );
    Ok(ContextCompiler::new(ContextPolicy::default()).compile(&execution))
}

/// Adds prior local turns without copying the currently submitted input.
///
/// - `execution`: Context receiving selected branch-local activities.
/// - `branch`: Exact branch whose local history is selected.
/// - `prompt`: Current instruction to omit from history.
/// - `continuation`: Whether ancestor history already came from another branch.
fn add_branch_history(
    execution: &mut BranchExecutionContext,
    branch: &Branch,
    prompt: &str,
    continuation: bool,
) {
    let current_turn = branch
        .turns
        .iter()
        .rev()
        .find(|turn| {
            turn.human
                .as_ref()
                .is_some_and(|human| human.text == prompt)
        })
        .map(|turn| turn.id);
    if !continuation {
        if let Some(current_turn) = current_turn {
            let handle = format!("turn:{current_turn}:human");
            execution
                .activities
                .retain(|activity| activity.handle.as_deref() != Some(handle.as_str()));
        }
        return;
    }
    for turn in &branch.turns {
        if Some(turn.id) == current_turn {
            continue;
        }
        if let Some(human) = &turn.human {
            execution.activities.push(
                ContextActivity::new(
                    turn.ordinal.saturating_mul(2),
                    "branch_local_human",
                    human.text.clone(),
                )
                .with_handle(format!("turn:{}:human", turn.id)),
            );
        }
        if let Some(agent) = &turn.agent {
            execution.activities.push(
                ContextActivity::new(
                    turn.ordinal.saturating_mul(2).saturating_add(1),
                    "branch_local_assistant",
                    agent.text.clone(),
                )
                .with_handle(format!("turn:{}:assistant", turn.id)),
            );
        }
    }
}

#[cfg(test)]
#[path = "agent_context_tests.rs"]
mod tests;

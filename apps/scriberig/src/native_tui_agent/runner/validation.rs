//! Validates upstream provider availability before automatic plan execution.

use scriberig_app::ApplicationError;

use super::super::NativeTuiAgentRunner;

/// Validates provider availability before a plan can begin automatic work.
///
/// Integration-only nodes may merge dependency candidates without a model
/// provider. Other pending nodes need an available upstream or local executor.
///
/// - `runner`: Configured upstream provider support and compatibility adapters.
/// - `plan`: Current durable plan state, when it exists.
/// - `action`: Requested lifecycle transition.
///
/// # Returns
/// Whether the plan may proceed to lifecycle mutation.
///
/// # Errors
/// Returns an unsupported-operation error when worker execution lacks a provider.
pub(super) fn validate_plan_provider(
    runner: &NativeTuiAgentRunner,
    plan: Option<&scriberig_session::Plan>,
    action: scriberig_app::PlanLifecycleAction,
) -> Result<(), ApplicationError> {
    let begins_execution = matches!(
        action,
        scriberig_app::PlanLifecycleAction::Start | scriberig_app::PlanLifecycleAction::Resume
    );
    let needs_provider = plan.is_some_and(|plan| {
        plan.nodes.iter().any(|node| {
            matches!(
                node.status,
                scriberig_session::PlanNodeStatus::Planned
                    | scriberig_session::PlanNodeStatus::Ready
            ) && node
                .expected_result
                .as_ref()
                .is_none_or(|expected| expected.kind != "project_candidate_merge")
        })
    });
    if begins_execution && needs_provider && runner.providers.is_empty() && !runner.upstream_enabled
    {
        return Err(ApplicationError::UnsupportedOperation {
            operation: "configure a provider connection before starting plan work".to_owned(),
        });
    }
    Ok(())
}

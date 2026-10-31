//! Executes terminal requests for explicit legacy selection recovery.

use crate::action::Action;
use crate::effect::Effect;
use crate::service::{ApplicationContext, TuiService};

/// Executes one legacy recovery request through the shared service boundary.
///
/// - `service`: Authorized local or remote recovery service.
/// - `context`: Caller authorization context.
/// - `effect`: Legacy recovery list or mapping request.
///
/// The presentation action describing the result.
/// # Returns
/// An action containing the safe outcome or error.
pub(super) fn execute<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    match effect {
        Effect::LegacyRecoveryList {
            session,
            selection_revision,
            plan_revisions,
        } => service
            .legacy_execution_records(context, session)
            .map(|records| {
                Action::RecoveryRecordsLoaded(recovery_records_message(
                    records,
                    selection_revision,
                    plan_revisions,
                ))
            })
            .unwrap_or_else(|error| failed(&error)),
        Effect::LegacyRecoveryMap {
            session,
            path,
            expected_revision,
            provider_id,
            model_id,
            thinking_level,
        } => {
            let result = if path == "/execution" {
                service.map_legacy_execution_selection(
                    context,
                    &scriberig_app::LegacyExecutionMappingRequest {
                        session,
                        expected_revision,
                        legacy_path: path,
                        provider_id,
                        model_id,
                        thinking_level,
                    },
                )
            } else {
                service
                    .map_legacy_plan_assignment(
                        context,
                        &scriberig_app::LegacyPlanAssignmentMappingRequest {
                            session,
                            legacy_path: path,
                            expected_plan_revision: expected_revision,
                            provider_id,
                            model_id,
                            thinking_level,
                        },
                    )
                    .map(|_| ())
            };
            result.map_or_else(
                |error| failed(&error),
                |()| {
                    Action::StatusMessage(
                        "Legacy choice mapped through the active model catalog.".to_owned(),
                    )
                },
            )
        }
        _ => Action::Noop,
    }
}

/// Formats sanitized records with the exact revisions needed for safe mapping.
///
/// - `records`: Sanitized protocol records returned by the application.
/// - `selection_revision`: Current session-selection revision.
/// - `plan_revisions`: Current plan identities and revisions.
///
/// # Returns
/// A concise review message with historical and claim targets marked read-only.
fn recovery_records_message(
    records: Vec<scriberig_protocol::LegacyExecutionRecordView>,
    selection_revision: u64,
    plan_revisions: Vec<(scriberig_core::EntityId, u64)>,
) -> String {
    if records.is_empty() {
        return "No preserved legacy execution choices need recovery.".to_owned();
    }
    let plans = plan_revisions
        .into_iter()
        .map(|(plan, revision)| (plan.to_string(), revision))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut lines = vec!["Preserved legacy choices:".to_owned()];
    for record in records {
        let target = match record.target {
            Some(target) if target.claim => format!(
                "worker claim for plan={} node={} · review only",
                target.plan_id, target.node_id
            ),
            Some(target) if target.revision.is_some() => plans
                .get(&target.plan_id)
                .map_or_else(
                    || format!(
                        "historical plan={} node={} source-revision={} · review only",
                        target.plan_id,
                        target.node_id,
                        target.revision.unwrap_or_default()
                    ),
                    |revision| format!(
                        "historical source-revision={} · current plan revision={revision} plan={} node={} · explicit remap available",
                        target.revision.unwrap_or_default(), target.plan_id, target.node_id
                    ),
                ),
            Some(target) => plans.get(&target.plan_id).map_or_else(
                || "current plan target unavailable · review only".to_owned(),
                |revision| {
                    format!(
                        "current plan revision={revision} plan={} node={}",
                        target.plan_id, target.node_id
                    )
                },
            ),
            None if record.path == "/execution" => {
                format!("session selection revision={selection_revision}")
            }
            None => "no current target · review only".to_owned(),
        };
        let value = crate::inspector::sanitize(&record.value.to_string())
            .chars()
            .take(160)
            .collect::<String>();
        lines.push(format!(
            "{} · {target} · value={value}",
            crate::inspector::sanitize(&record.path)
        ));
    }
    lines.push(
        "Map with /recover map <exact-path> <observed-revision> <provider> <model> [thinking]."
            .to_owned(),
    );
    lines.join("\n")
}

/// Converts an application recovery failure into a safe presentation action.
///
/// - `error`: Structured application failure.
///
/// The operation failure action with sanitized message text.
/// # Returns
/// An action safe to display in the terminal.
fn failed(error: &scriberig_app::ApplicationError) -> Action {
    Action::OperationFailed {
        request_id: 0,
        category: error.category().to_owned(),
        message: crate::inspector::sanitize(&error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::recovery_records_message;
    use scriberig_protocol::LegacyExecutionRecordView;

    /// Shows active revisions and distinguishes recoverable historical assignments.
    #[test]
    fn recovery_listing_marks_eligible_and_historical_targets() {
        let active_plan = scriberig_core::EntityId::from_bytes([8; 16]);
        let active_node = scriberig_core::EntityId::from_bytes([9; 16]);
        let session_execution = LegacyExecutionRecordView {
            path: "/execution".to_owned(),
            value: serde_json::json!({"model_id": "old-model"}),
            target: None,
        };
        let historical = LegacyExecutionRecordView {
            path: "/plans/old/nodes/node/model".to_owned(),
            value: serde_json::json!({"model_id": "archived-model"}),
            target: Some(scriberig_protocol::LegacyExecutionTargetView {
                plan_id: "old-plan".to_owned(),
                node_id: "old-node".to_owned(),
                revision: Some(2),
                claim: false,
            }),
        };
        let active_historical = LegacyExecutionRecordView {
            path: "/plans/0/history/0/nodes/0/assignment".to_owned(),
            value: serde_json::json!({"model_id": "archived-model"}),
            target: Some(scriberig_protocol::LegacyExecutionTargetView {
                plan_id: active_plan.to_string(),
                node_id: active_node.to_string(),
                revision: Some(2),
                claim: false,
            }),
        };
        let message = recovery_records_message(
            vec![session_execution, historical, active_historical],
            5,
            vec![(active_plan, 7)],
        );
        assert!(message.contains("/execution · session selection revision=5"));
        assert!(
            message
                .contains("historical plan=old-plan node=old-node source-revision=2 · review only")
        );
        assert!(message.contains(
            &format!(
                "historical source-revision=2 · current plan revision=7 plan={active_plan} node={active_node} · explicit remap available"
            )
        ));
        assert!(message.contains("old-model"));
        assert!(message.contains("archived-model"));
    }
}

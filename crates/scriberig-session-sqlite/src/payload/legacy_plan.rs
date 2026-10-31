//! Imports legacy plan assignments with exact semantic owner metadata.

use scriberig_core::EntityId;
use scriberig_session::{LegacyExecutionRecord, LegacyExecutionTarget};
use serde_json::Value;

/// Moves legacy profile/model assignments out of durable plan structures.
///
/// - `session`: Untrusted JSON session object being imported.
/// - `legacy`: Destination for exact legacy assignment objects.
pub(super) fn import_plan_assignments(
    session: &mut Value,
    legacy: &mut Vec<LegacyExecutionRecord>,
) {
    let Some(plans) = session.get_mut("plans").and_then(Value::as_array_mut) else {
        return;
    };
    for (plan_index, plan) in plans.iter_mut().enumerate() {
        let plan_id = plan
            .get("id")
            .cloned()
            .and_then(|value| serde_json::from_value::<EntityId>(value).ok());
        if let Some(nodes) = plan.get_mut("nodes").and_then(Value::as_array_mut) {
            import_nodes(
                nodes,
                &format!("/plans/{plan_index}/nodes"),
                legacy,
                plan_id,
                None,
            );
        }
        if let Some(history) = plan.get_mut("history").and_then(Value::as_array_mut) {
            for (revision_index, revision) in history.iter_mut().enumerate() {
                let plan_revision = revision.get("revision").and_then(Value::as_u64);
                if let Some(nodes) = revision.get_mut("nodes").and_then(Value::as_array_mut) {
                    import_nodes(
                        nodes,
                        &format!("/plans/{plan_index}/history/{revision_index}/nodes"),
                        legacy,
                        plan_id,
                        plan_revision,
                    );
                }
            }
        }
    }
}

/// Clears legacy assignment objects while recording their exact JSON paths.
///
/// - `nodes`: Plan node JSON values.
/// - `base_path`: JSON pointer to the node array.
/// - `legacy`: Destination for exact legacy assignment objects.
/// - `plan_id`: Stable plan identity when the payload identity can be parsed.
/// - `revision`: Historical plan revision, or `None` for the current plan.
fn import_nodes(
    nodes: &mut [Value],
    base_path: &str,
    legacy: &mut Vec<LegacyExecutionRecord>,
    plan_id: Option<EntityId>,
    revision: Option<u64>,
) {
    for (node_index, node) in nodes.iter_mut().enumerate() {
        let node_path = format!("{base_path}/{node_index}");
        let target = plan_id
            .zip(
                node.get("id")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<EntityId>(value).ok()),
            )
            .map(|(plan_id, node_id)| LegacyExecutionTarget {
                plan_id,
                node_id,
                revision,
                claim: false,
            });
        import_assignment(
            node,
            &format!("{node_path}/assignment"),
            legacy,
            target.as_ref(),
        );
        if let Some(claim) = node.get_mut("claim") {
            if let Some(mut claim_target) = target {
                claim_target.claim = true;
                import_assignment(
                    claim,
                    &format!("{node_path}/claim/assignment"),
                    legacy,
                    Some(&claim_target),
                );
            } else {
                import_assignment(
                    claim,
                    &format!("{node_path}/claim/assignment"),
                    legacy,
                    None,
                );
            }
        }
    }
}

/// Retains and clears one legacy `{profile, model}` assignment.
///
/// - `owner`: Node or worker-claim object that contains an assignment.
/// - `path`: Exact JSON pointer to the assignment field.
/// - `legacy`: Destination for the exact old assignment object.
/// - `target`: Stable plan and node identity when available.
fn import_assignment(
    owner: &mut Value,
    path: &str,
    legacy: &mut Vec<LegacyExecutionRecord>,
    target: Option<&LegacyExecutionTarget>,
) {
    let Some(owner) = owner.as_object_mut() else {
        return;
    };
    let Some(assignment) = owner.get("assignment") else {
        return;
    };
    let Some(object) = assignment.as_object() else {
        return;
    };
    let has_current_pair = object.get("provider_id").and_then(Value::as_str).is_some()
        && object.get("model_id").and_then(Value::as_str).is_some();
    if has_current_pair && object.contains_key("profile") {
        legacy.push(LegacyExecutionRecord {
            path: path.to_owned(),
            value: Value::Object(super::selected_fields(
                object,
                &["profile", "model", "thinking_level"],
            )),
            target: target.cloned(),
        });
        return;
    }
    if has_current_pair || !object.contains_key("profile") {
        return;
    }
    legacy.push(LegacyExecutionRecord {
        path: path.to_owned(),
        value: Value::Object(super::selected_fields(
            object,
            &["profile", "model", "thinking_level"],
        )),
        target: target.cloned(),
    });
    owner.insert("assignment".to_owned(), Value::Null);
}

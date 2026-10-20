//! Decodes versioned SQLite session payloads and preserves retired selections.

use scriberig_session::Session;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const LEGACY_PAYLOAD_VERSION: u32 = 1;

/// Retains one exact legacy model selection that cannot be resolved safely.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct LegacyExecutionRecord {
    /// JSON pointer locating the original value in the prior session payload.
    pub path: String,
    /// Allowlisted historical selection fields; arbitrary properties are removed.
    pub value: Value,
}

/// Holds a decoded session and any unresolved legacy execution choices.
pub(crate) struct DecodedSession {
    /// Current typed session graph.
    pub session: Session,
    /// Exact retired selection values retained for operator recovery.
    pub legacy_execution: Vec<LegacyExecutionRecord>,
    /// Whether the original payload must be rewritten to remove unknown data.
    pub requires_rewrite: bool,
}

/// Versioned envelope for sessions containing sanitized legacy selections.
///
/// # Fields
/// - `version`: Version of the recovery envelope.
/// - `session`: Current typed session graph.
/// - `legacy_execution`: Allowlisted legacy choices retained for recovery.
#[derive(Deserialize, Serialize)]
struct VersionedSessionPayload {
    /// Version of this SQLite-only recovery envelope.
    version: u32,
    /// Session graph used by current `ScribeRig` readers.
    session: Session,
    /// Legacy selection values that could not be mapped to upstream IDs.
    legacy_execution: Vec<LegacyExecutionRecord>,
}

/// Decodes a current payload or imports a known legacy selection shape.
///
/// - `payload`: JSON bytes read from `session_records`.
///
/// The typed session and any exact legacy choices retained for recovery.
///
/// Returns an error when the payload is neither current nor a supported
/// legacy-selection record.
///
/// # Returns
/// The decoded session and rewrite metadata.
///
/// # Errors
/// Returns a diagnostic when JSON decoding or session normalization fails.
pub(crate) fn decode(payload: &[u8]) -> Result<DecodedSession, String> {
    let value: Value = serde_json::from_slice(payload).map_err(|error| error.to_string())?;
    if value.get("version").and_then(Value::as_u64) == Some(u64::from(LEGACY_PAYLOAD_VERSION)) {
        let envelope: VersionedSessionPayload =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        let legacy_execution = sanitize_legacy_records(envelope.legacy_execution);
        let normalized = normalized_value(&envelope.session, &legacy_execution)?;
        return Ok(DecodedSession {
            session: envelope.session,
            legacy_execution,
            requires_rewrite: normalized != value,
        });
    }

    let mut imported = value.clone();
    let mut legacy_execution = Vec::new();
    import_session_selection(&mut imported, &mut legacy_execution);
    import_plan_assignments(&mut imported, &mut legacy_execution);
    let session = serde_json::from_value(imported).map_err(|error| error.to_string())?;
    let normalized = normalized_value(&session, &legacy_execution)?;
    Ok(DecodedSession {
        session,
        legacy_execution,
        requires_rewrite: normalized != value,
    })
}

/// Encodes a normal session directly or wraps it with unresolved legacy data.
///
/// - `session`: Current typed session graph.
/// - `legacy_execution`: Exact older choices that remain unresolved.
///
/// JSON bytes suitable for `session_records.payload`.
///
/// Returns an error when serialization fails.
///
/// # Returns
/// The encoded session payload.
///
/// # Errors
/// Returns a diagnostic when JSON serialization fails.
pub(crate) fn encode(
    session: &Session,
    legacy_execution: &[LegacyExecutionRecord],
) -> Result<Vec<u8>, String> {
    if legacy_execution.is_empty() {
        return serde_json::to_vec(session).map_err(|error| error.to_string());
    }
    let envelope = VersionedSessionPayload {
        version: LEGACY_PAYLOAD_VERSION,
        session: session.clone(),
        legacy_execution: legacy_execution.to_vec(),
    };
    serde_json::to_vec(&envelope).map_err(|error| error.to_string())
}

/// Moves a retired session choice into the recoverable payload extension.
///
/// - `session`: Untrusted JSON session object being imported.
/// - `legacy`: Destination for exact legacy selection objects.
fn import_session_selection(session: &mut Value, legacy: &mut Vec<LegacyExecutionRecord>) {
    let Some(execution) = session.get_mut("execution").and_then(Value::as_object_mut) else {
        return;
    };
    let has_current_pair = execution
        .get("provider_id")
        .and_then(Value::as_str)
        .is_some()
        && execution.get("model_id").and_then(Value::as_str).is_some();
    let has_legacy_choice = execution
        .get("model_profile")
        .is_some_and(|value| !value.is_null())
        || (!has_current_pair && execution.get("model").is_some_and(|value| !value.is_null()));
    if !has_legacy_choice {
        return;
    }

    if has_current_pair {
        legacy.push(LegacyExecutionRecord {
            path: "/execution".to_owned(),
            value: Value::Object(selected_fields(
                execution,
                &["model_profile", "model", "thinking_level"],
            )),
        });
        return;
    }
    legacy.push(LegacyExecutionRecord {
        path: "/execution".to_owned(),
        value: Value::Object(selected_fields(
            execution,
            &["model_profile", "model", "thinking_level", "revision"],
        )),
    });
    let revision = execution
        .get("revision")
        .cloned()
        .unwrap_or_else(|| Value::from(0));
    *execution = current_empty_selection(revision);
}

/// Moves legacy profile/model assignments out of durable plan structures.
///
/// - `session`: Untrusted JSON session object being imported.
/// - `legacy`: Destination for exact legacy assignment objects.
fn import_plan_assignments(session: &mut Value, legacy: &mut Vec<LegacyExecutionRecord>) {
    let Some(plans) = session.get_mut("plans").and_then(Value::as_array_mut) else {
        return;
    };
    for (plan_index, plan) in plans.iter_mut().enumerate() {
        if let Some(nodes) = plan.get_mut("nodes").and_then(Value::as_array_mut) {
            import_nodes(nodes, &format!("/plans/{plan_index}/nodes"), legacy);
        }
        if let Some(history) = plan.get_mut("history").and_then(Value::as_array_mut) {
            for (revision_index, revision) in history.iter_mut().enumerate() {
                if let Some(nodes) = revision.get_mut("nodes").and_then(Value::as_array_mut) {
                    import_nodes(
                        nodes,
                        &format!("/plans/{plan_index}/history/{revision_index}/nodes"),
                        legacy,
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
fn import_nodes(nodes: &mut [Value], base_path: &str, legacy: &mut Vec<LegacyExecutionRecord>) {
    for (node_index, node) in nodes.iter_mut().enumerate() {
        let node_path = format!("{base_path}/{node_index}");
        import_assignment(node, &format!("{node_path}/assignment"), legacy);
        if let Some(claim) = node.get_mut("claim") {
            import_assignment(claim, &format!("{node_path}/claim/assignment"), legacy);
        }
    }
}

/// Retains and clears one legacy `{profile, model}` assignment.
///
/// - `owner`: Node or worker-claim object that contains an assignment.
/// - `path`: Exact JSON pointer to the assignment field.
/// - `legacy`: Destination for the exact old assignment object.
fn import_assignment(owner: &mut Value, path: &str, legacy: &mut Vec<LegacyExecutionRecord>) {
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
            value: Value::Object(selected_fields(
                object,
                &["profile", "model", "thinking_level"],
            )),
        });
        return;
    }
    if has_current_pair || !object.contains_key("profile") {
        return;
    }
    legacy.push(LegacyExecutionRecord {
        path: path.to_owned(),
        value: Value::Object(selected_fields(
            object,
            &["profile", "model", "thinking_level"],
        )),
    });
    owner.insert("assignment".to_owned(), Value::Null);
}

/// Keeps only fields declared by the historical selection data types.
///
/// - `value`: Historical JSON object.
/// - `allowed`: Exact historical field names safe for selection recovery.
///
/// A projection without arbitrary extension or credential-bearing fields.
///
/// # Returns
/// The allowlisted fields with unsafe values omitted.
fn selected_fields(value: &Map<String, Value>, allowed: &[&str]) -> Map<String, Value> {
    let mut selected = Map::new();
    for key in allowed {
        if let Some(value) = value.get(*key) {
            let safe_value = if *key == "revision" {
                value.as_u64().map(Value::from)
            } else if value.is_null() || value.as_str().is_some() {
                Some(value.clone())
            } else {
                None
            };
            if let Some(value) = safe_value {
                selected.insert((*key).to_owned(), value);
            }
        }
    }
    selected
}

/// Redacts unsupported fields from already-versioned recovery records.
///
/// - `records`: Recovery records from a persisted envelope.
///
/// Records limited to the known legacy selection schema.
///
/// # Returns
/// Sanitized records with unsupported paths and fields removed.
fn sanitize_legacy_records(records: Vec<LegacyExecutionRecord>) -> Vec<LegacyExecutionRecord> {
    records
        .into_iter()
        .filter_map(|mut record| {
            let allowed = if record.path == "/execution" {
                &["model_profile", "model", "thinking_level", "revision"][..]
            } else if is_assignment_path(&record.path) {
                &["profile", "model", "thinking_level"][..]
            } else {
                return None;
            };
            if let Value::Object(object) = &record.value {
                record.value = Value::Object(selected_fields(object, allowed));
            } else {
                record.value = Value::Object(Map::new());
            }
            record
                .value
                .as_object()
                .is_some_and(|object| !object.is_empty())
                .then_some(record)
        })
        .collect()
}

/// Serializes the canonical current payload after dropping unknown fields.
///
/// - `session`: Typed `ScribeRig` session graph.
/// - `legacy_execution`: Allowlisted retired choices retained for recovery.
///
/// Canonical JSON used to detect raw extension data that must be scrubbed.
/// # Returns
/// The normalized payload value.
/// # Errors
/// Returns a serialization error when the typed payload cannot be encoded.
fn normalized_value(
    session: &Session,
    legacy_execution: &[LegacyExecutionRecord],
) -> Result<Value, String> {
    if legacy_execution.is_empty() {
        serde_json::to_value(session).map_err(|error| error.to_string())
    } else {
        serde_json::to_value(VersionedSessionPayload {
            version: LEGACY_PAYLOAD_VERSION,
            session: session.clone(),
            legacy_execution: legacy_execution.to_vec(),
        })
        .map_err(|error| error.to_string())
    }
}

/// Accepts only JSON pointers generated for known plan assignment locations.
///
/// - `path`: Candidate recovery path from a versioned payload.
///
/// `true` when the path has the expected plan/node/assignment shape.
///
/// # Returns
/// Whether the path points to a supported plan assignment location.
fn is_assignment_path(path: &str) -> bool {
    let parts = path.split('/').collect::<Vec<_>>();
    let is_index =
        |value: &str| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    match parts.as_slice() {
        ["", "plans", plan, "nodes", node, "assignment"] => is_index(plan) && is_index(node),
        ["", "plans", plan, "nodes", node, "claim", "assignment"] => {
            is_index(plan) && is_index(node)
        }
        [
            "",
            "plans",
            plan,
            "history",
            revision,
            "nodes",
            node,
            "assignment",
        ]
        | [
            "",
            "plans",
            plan,
            "history",
            revision,
            "nodes",
            node,
            "claim",
            "assignment",
        ] => is_index(plan) && is_index(revision) && is_index(node),
        _ => false,
    }
}

/// Builds the typed empty selection used while the old choice is unresolved.
///
/// - `revision`: Preserved selection revision from the legacy record.
///
/// A current-format session selection with no resolved model choice.
///
/// # Returns
/// The current selection fields with the supplied revision.
fn current_empty_selection(revision: Value) -> Map<String, Value> {
    let mut selection = Map::new();
    selection.insert("revision".to_owned(), revision);
    selection.insert("provider_id".to_owned(), Value::Null);
    selection.insert("model_id".to_owned(), Value::Null);
    selection.insert("thinking_level".to_owned(), Value::Null);
    selection.insert("history".to_owned(), Value::Array(Vec::new()));
    selection
}

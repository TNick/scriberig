//! Verifies imports retain retired model selections through SQLite writes.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ModelAssignment, Plan, PlanNode, PlanNodeStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

/// Preserves a retired session model choice through selection compare-and-swap.
#[test]
fn legacy_session_choice_survives_load_and_selection_cas() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-legacy-selection-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([71; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::from_bytes([72; 16]), "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut raw = serde_json::to_value(&session).expect("serialize session");
    raw["execution"] = serde_json::json!({
        "revision": 4,
        "model_profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep",
        "credential": "UNSAFE_LEGACY_MARKER",
        "history": []
    });
    raw["credential_extension"] = serde_json::json!({
        "api_key": "UNSAFE_TOP_LEVEL_MARKER"
    });
    insert_raw_session(&path, session_id, &raw);

    let mut loaded = store
        .load(session_id)
        .expect("load imported session")
        .expect("session exists");
    assert_eq!(loaded.execution.revision, 4);
    assert_eq!(loaded.execution.model_id, None);
    let stored = read_raw_session_payload(&path, session_id);
    assert!(!stored.to_string().contains("UNSAFE_LEGACY_MARKER"));
    assert!(!stored.to_string().contains("UNSAFE_TOP_LEVEL_MARKER"));
    let legacy = store
        .legacy_execution_records(session_id)
        .expect("read retained legacy selection");
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].path, "/execution");
    assert_eq!(legacy[0].value["model_profile"], "retired-profile");
    assert_eq!(legacy[0].value["model"], "retired-model");
    assert!(
        !serde_json::to_string(&legacy)
            .expect("serialize sanitized recovery data")
            .contains("UNSAFE_LEGACY_MARKER")
    );

    loaded
        .execution
        .replace(
            4,
            Some("provider-current".to_owned()),
            Some("model-current".to_owned()),
            None,
        )
        .expect("replace selection");
    store
        .compare_and_swap_execution_selection(&loaded, 4)
        .expect("save selection");

    let reloaded = store
        .load(session_id)
        .expect("reload session")
        .expect("session remains");
    assert_eq!(
        reloaded.execution.model_id.as_deref(),
        Some("model-current")
    );
    assert_eq!(
        store
            .legacy_execution_records(session_id)
            .expect("read preserved legacy choice"),
        legacy
    );
    store
        .discard_legacy_execution_records(session_id)
        .expect("discard after explicit reset");
    assert_eq!(
        store
            .legacy_execution_records(session_id)
            .expect("read cleared recovery data"),
        Vec::new()
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Scrubs unknown credential-like fields when listing legacy session rows.
#[test]
fn legacy_list_scrubs_unknown_credential_extension_fields() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-legacy-list-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([79; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::from_bytes([80; 16]), "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut raw = serde_json::to_value(&session).expect("serialize session");
    raw["credential_extension"] = serde_json::json!({
        "api_key": "UNSAFE_LIST_MARKER"
    });
    insert_raw_session(&path, session_id, &raw);

    assert_eq!(store.list().expect("list sessions").len(), 1);
    let stored = read_raw_session_payload(&path, session_id);
    assert!(!stored.to_string().contains("UNSAFE_LIST_MARKER"));

    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Retains a retired profile reference without changing a valid current pair.
#[test]
fn current_selection_pair_survives_retired_profile_import() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-current-pair-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([77; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::from_bytes([78; 16]), "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut raw = serde_json::to_value(&session).expect("serialize session");
    raw["execution"] = serde_json::json!({
        "revision": 5,
        "provider_id": "provider-current",
        "model_id": "model-current",
        "thinking_level": "high",
        "model_profile": "retired-profile",
        "model": "retired-model",
        "credential": "UNSAFE_LEGACY_MARKER",
        "history": []
    });
    insert_raw_session(&path, session_id, &raw);

    let loaded = store
        .load(session_id)
        .expect("load imported session")
        .expect("session exists");
    assert_eq!(
        loaded.execution.provider_id.as_deref(),
        Some("provider-current")
    );
    assert_eq!(loaded.execution.model_id.as_deref(), Some("model-current"));
    let legacy = store
        .legacy_execution_records(session_id)
        .expect("read retired profile reference");
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].value["model_profile"], "retired-profile");
    assert_eq!(legacy[0].value["model"], "retired-model");
    assert!(
        !serde_json::to_string(&legacy)
            .expect("serialize sanitized recovery data")
            .contains("UNSAFE_LEGACY_MARKER")
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Preserves legacy plan assignments while plan CAS writes a current assignment.
#[test]
fn legacy_plan_assignment_survives_load_and_plan_cas() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-legacy-plan-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([73; 16]);
    let root = EntityId::from_bytes([74; 16]);
    let node_id = EntityId::from_bytes([75; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let mut plan = Plan::new(
        EntityId::from_bytes([76; 16]),
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root },
        "legacy plan assignment",
    )
    .expect("create plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
        title: "work".to_owned(),
        purpose: "complete work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: Some(current_assignment()),
        claim: None,
        claim_generation: 0,
    })
    .expect("add plan node");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut raw = serde_json::to_value(&session).expect("serialize session");
    raw["plans"][0]["nodes"][0]["assignment"] = legacy_assignment();
    for node in raw["plans"][0]["history"]
        .as_array_mut()
        .expect("plan history")
        .iter_mut()
        .filter_map(|revision| revision["nodes"].as_array_mut())
        .flat_map(|nodes| nodes.iter_mut())
    {
        if node["id"] == serde_json::to_value(node_id).expect("encode node ID") {
            node["assignment"] = legacy_assignment();
        }
    }
    insert_raw_session(&path, session_id, &raw);

    let mut loaded = store
        .load(session_id)
        .expect("load imported session")
        .expect("session exists");
    assert_eq!(loaded.plans[0].nodes[0].assignment, None);
    let legacy = store
        .legacy_execution_records(session_id)
        .expect("read retained plan assignments");
    assert!(legacy.iter().any(|record| {
        record.path == "/plans/0/nodes/0/assignment"
            && record.value["profile"] == "retired-profile"
            && record.value["model"] == "retired-model"
    }));
    assert!(
        !serde_json::to_string(&legacy)
            .expect("serialize sanitized recovery data")
            .contains("UNSAFE_LEGACY_MARKER")
    );

    let expected_revision = loaded.plans[0].revision;
    loaded.plans[0]
        .set_assignment(node_id, Some(current_assignment()))
        .expect("set current assignment");
    store
        .compare_and_swap_plan(&loaded, loaded.plans[0].id, expected_revision)
        .expect("save plan assignment");
    assert_eq!(
        store
            .legacy_execution_records(session_id)
            .expect("read preserved legacy assignments"),
        legacy
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Inserts serialized JSON using the legacy session-record schema.
///
/// - `path`: SQLite database path initialized by the session store.
/// - `session`: Stable session identity for the row key.
/// - `payload`: Legacy session payload to store.
fn insert_raw_session(path: &std::path::Path, session: EntityId, payload: &serde_json::Value) {
    let connection = Connection::open(path).expect("open test connection");
    connection
        .execute(
            "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)",
            params![
                session.as_bytes().as_slice(),
                serde_json::to_vec(payload).expect("serialize legacy payload")
            ],
        )
        .expect("insert legacy record");
}

/// Reads one stored SQLite payload for migration assertions.
///
/// - `path`: SQLite database containing the session row.
/// - `session`: Stable identity of the stored row.
///
/// The raw decoded JSON payload.
/// # Returns
/// The stored payload value.
fn read_raw_session_payload(path: &std::path::Path, session: EntityId) -> serde_json::Value {
    let connection = Connection::open(path).expect("open payload connection");
    let payload = connection
        .query_row(
            "SELECT payload FROM session_records WHERE session_id=?1",
            params![session.as_bytes().as_slice()],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("read stored payload");
    serde_json::from_slice(&payload).expect("decode stored payload")
}

/// Builds an upstream-addressed model assignment for a plan update.
///
/// # Returns
/// A provider/model pair independent from the retired profile catalog.
fn current_assignment() -> ModelAssignment {
    ModelAssignment {
        provider_id: "provider-current".to_owned(),
        model_id: "model-current".to_owned(),
        thinking_level: None,
    }
}

/// Builds one retired profile/model assignment from the former catalog.
///
/// # Returns
/// The exact JSON object expected from legacy persisted plans.
fn legacy_assignment() -> serde_json::Value {
    serde_json::json!({
        "profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep",
        "credential": "UNSAFE_LEGACY_MARKER"
    })
}

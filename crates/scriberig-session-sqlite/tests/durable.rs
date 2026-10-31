//! Verifies session graphs survive repository reopen and preserve child state.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, CacheDisposition, ModelAssignment, ModelInvocation, Plan,
    PlanNode, PlanNodeStatus, SelectionStoreError, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

/// Lists complete durable sessions in stable identity order.
#[test]
fn session_store_lists_durable_graphs() {
    let path =
        std::env::temp_dir().join(format!("scriberig-session-list-{}.sqlite", EntityId::new()));
    let store = SqliteSessionStore::open(&path).expect("open store");
    for id in [EntityId::from_bytes([2; 16]), EntityId::from_bytes([1; 16])] {
        store
            .save(&Session {
                id,
                branches: vec![Branch::new(EntityId::new(), "root").expect("root")],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: Vec::new(),
                execution: scriberig_session::SessionExecutionSelection::default(),
            })
            .expect("save session");
    }

    let sessions = store.list().expect("list sessions");

    assert_eq!(
        sessions
            .iter()
            .map(|session| session.id)
            .collect::<Vec<_>>(),
        [EntityId::from_bytes([1; 16]), EntityId::from_bytes([2; 16])]
    );
    let _ = std::fs::remove_file(path);
}

/// Removes exactly the requested durable session while retaining other sessions.
#[test]
fn session_store_removes_one_exact_graph() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-remove-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("open store");
    let ids = [
        EntityId::from_bytes([11; 16]),
        EntityId::from_bytes([12; 16]),
    ];
    for id in ids {
        store
            .save(&Session {
                id,
                branches: vec![Branch::new(EntityId::new(), "root").expect("root")],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: Vec::new(),
                execution: scriberig_session::SessionExecutionSelection::default(),
            })
            .expect("save session");
    }

    store.remove(ids[0]).expect("remove session");

    assert!(store.load(ids[0]).expect("load removed session").is_none());
    assert!(store.load(ids[1]).expect("load retained session").is_some());
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Verifies a complete session graph round-trips through SQLite.
#[test]
fn session_graph_survives_reopen() {
    let path = std::env::temp_dir().join(format!("scriberig-session-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([1; 16]);
    let root_id = EntityId::from_bytes([2; 16]);
    let child_id = EntityId::from_bytes([3; 16]);
    {
        let store = SqliteSessionStore::open(&path).expect("open store");
        let mut root = Branch::new(root_id, "root").expect("root");
        root.model_invocations.push(ModelInvocation {
            id: EntityId::from_bytes([4; 16]),
            branch: root_id,
            operation: None,
            ordinal: 1,
            agent_profile: None,
            context_fingerprint: Some("context-fingerprint".to_owned()),
            provider_profile: Some("provider-a".to_owned()),
            provider_id: None,
            model: Some("model-a".to_owned()),
            model_profile: Some("profile-a".to_owned()),
            provider_model_id: Some("native-model-a".to_owned()),
            thinking_level: Some("careful".to_owned()),
            request_fingerprint: Some("request-fingerprint".to_owned()),
            cache: CacheDisposition::NotUsed,
            output: None,
            request: None,
        });
        let mut session = Session {
            id: session_id,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };
        session
            .add_child_branch(child_id, root_id, "child", "inspect")
            .expect("child");
        session
            .execution
            .replace(
                0,
                Some("provider-a".to_owned()),
                Some("model-a".to_owned()),
                Some("careful".to_owned()),
            )
            .expect("execution selection");
        store.save(&session).expect("save session");
    }
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = reopened
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    assert_eq!(loaded.id, session_id);
    assert_eq!(loaded.branches.len(), 2);
    assert_eq!(loaded.branches[0].children, vec![child_id]);
    assert_eq!(loaded.execution.revision, 1);
    assert_eq!(loaded.execution.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(loaded.execution.model_id.as_deref(), Some("model-a"));
    assert_eq!(loaded.execution.thinking_level.as_deref(), Some("careful"));
    assert_eq!(
        loaded.branches[0].model_invocations[0]
            .provider_model_id
            .as_deref(),
        Some("native-model-a")
    );
    assert_eq!(
        loaded.branches[0].model_invocations[0]
            .thinking_level
            .as_deref(),
        Some("careful")
    );
    let _ = std::fs::remove_file(path);
}

#[path = "durable_parts/plan_dependencies.rs"]
mod plan_dependencies;

#[path = "durable_parts/compact_plan_view.rs"]
mod compact_plan_view;
#[path = "durable_parts/legacy_plan_migration.rs"]
mod legacy_plan_migration;
#[path = "durable_parts/legacy_plan_migration_claims.rs"]
mod legacy_plan_migration_claims;
#[path = "durable_parts/legacy_plan_migration_recovery.rs"]
mod legacy_plan_migration_recovery;
#[path = "durable_parts/part1.rs"]
mod part1;
#[path = "durable_parts/plan_attempt_recovery.rs"]
mod plan_attempt_recovery;
#[path = "durable_parts/plan_node_index.rs"]
mod plan_node_index;
#[path = "durable_parts/plan_projection.rs"]
mod plan_projection;
#[path = "durable_parts/plan_rollup_sparse.rs"]
mod plan_rollup_sparse;
#[path = "durable_parts/plan_settlement_rollback.rs"]
mod plan_settlement_rollback;
#[path = "durable_parts/request_provenance.rs"]
mod request_provenance;

#[path = "durable_parts/busy_timeout.rs"]
mod busy_timeout;
#[path = "durable_parts/claim_revision_scope.rs"]
mod claim_revision_scope;
#[path = "durable_parts/incremental_rollup.rs"]
mod incremental_rollup;
#[path = "durable_parts/lifecycle_revisions.rs"]
mod lifecycle_revisions;
#[path = "durable_parts/part2.rs"]
mod part2;
#[path = "durable_parts/plan_history_migration.rs"]
mod plan_history_migration;
#[path = "durable_parts/plan_scope/mod.rs"]
mod plan_scope;
#[path = "durable_parts/rollup_index.rs"]
mod rollup_index;

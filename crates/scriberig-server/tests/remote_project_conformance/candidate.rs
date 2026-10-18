//! Exercises canonical multi-file candidates and their public durable result references.

use std::fs;

use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_server::{InProcessProjectTransport, InProcessTransport};
use scriberig_session::{
    Branch, BranchResult, BranchStatus, Session, SessionExecutionSelection, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use serde_json::json;

use super::TestApplication;

/// Verifies aggregate proposal, detail, and validation through the remote client.
///
/// # Parameters
/// - `client`: Public project client.
/// - `root`: Live project root that must remain unchanged before Apply.
/// - `project`: Exact project identity.
/// - `snapshot`: Exact candidate base snapshot.
/// - `source`: Base source bytes as UTF-8.
/// - `super`: Parent conformance test module.
///
/// # Returns
/// Candidate identity and successor snapshot after aggregate Apply.
pub(super) fn exercise_remote_aggregate_candidate(
    client: &Client<InProcessProjectTransport<TestApplication>>,
    root: &std::path::Path,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
    source: &str,
) -> (String, String) {
    let aggregate = client
        .project(
            "propose-change",
            json!({
                "project": project.to_string(),
                "base_snapshot": snapshot.to_string(),
                "effects": [
                    {
                        "path": "src/lib.rs",
                        "kind": "update",
                        "content": source.replace("left + right", "left.saturating_add(right)").into_bytes()
                    },
                    {
                        "path": "src/generated.rs",
                        "kind": "create",
                        "content": b"pub fn generated() -> u8 { 1 }\n".to_vec()
                    }
                ]
            }),
        )
        .expect("remote aggregate candidate");
    assert_eq!(aggregate["base"], snapshot.to_string());
    assert_eq!(aggregate["effects"].as_array().expect("effects").len(), 2);
    assert!(!root.join("src/generated.rs").exists());
    let inspected = client
        .project("candidate", json!({"candidate": aggregate["candidate"]}))
        .expect("durable candidate detail");
    assert_eq!(inspected["effects"][0]["path"], "src/generated.rs");
    assert_eq!(
        inspected["effects"][0]["newBytes"],
        json!(b"pub fn generated() -> u8 { 1 }\n".to_vec())
    );
    assert_eq!(inspected["effects"][1]["path"], "src/lib.rs");
    assert_eq!(
        inspected["effects"][1]["oldBytes"],
        json!(source.as_bytes())
    );
    assert_eq!(
        inspected["effects"][1]["newBytes"],
        json!(
            source
                .replace("left + right", "left.saturating_add(right)")
                .into_bytes()
        )
    );
    let validated = client
        .project(
            "validate-candidate",
            json!({"candidate": aggregate["candidate"]}),
        )
        .expect("isolated candidate validation");
    assert_eq!(validated["validation"], "passed");
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("live source unchanged"),
        source
    );
    let applied = client
        .project("apply", json!({"candidate": aggregate["candidate"]}))
        .expect("apply aggregate candidate");
    assert_eq!(applied["materialization"], "applied");
    assert_eq!(applied["paths"].as_array().expect("paths").len(), 2);
    let effect_detail = client
        .project("candidate", json!({"candidate": aggregate["candidate"]}))
        .expect("durable effect detail");
    assert_eq!(
        effect_detail["effectPaths"]
            .as_array()
            .expect("path receipts")
            .len(),
        2
    );
    assert!(
        effect_detail["effectPaths"]
            .as_array()
            .expect("path receipts")
            .iter()
            .all(|receipt| receipt["state"] == "applied")
    );
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("materialized source"),
        source.replace("left + right", "left.saturating_add(right)")
    );
    assert_eq!(
        fs::read(root.join("src/generated.rs")).expect("materialized candidate file"),
        b"pub fn generated() -> u8 { 1 }\n"
    );
    (
        aggregate["candidate"]
            .as_str()
            .expect("candidate id")
            .to_owned(),
        applied["successor"]
            .as_str()
            .expect("successor snapshot")
            .to_owned(),
    )
}

/// Builds a durable child result linked to canonical project transition handles.
///
/// # Parameters
/// - `candidate`: Canonical candidate identity returned by the project API.
/// - `successor`: Applied snapshot identity returned by the project API.
/// - `super`: Parent conformance test module.
///
/// # Returns
/// A session graph containing a completed implementation branch.
pub(super) fn candidate_branch_result(candidate: &str, successor: &str) -> Session {
    let session_id = EntityId::new();
    let root_id = EntityId::new();
    let child_id = EntityId::new();
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root_id, "root").expect("root branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    session
        .add_child_branch(child_id, root_id, "PHP implementation", "produce candidate")
        .expect("implementation child");
    let branch = session
        .branches
        .iter_mut()
        .find(|branch| branch.id == child_id)
        .expect("child branch");
    branch.status = BranchStatus::Completed;
    branch.result = Some(BranchResult {
        id: EntityId::new(),
        branch: child_id,
        status: BranchStatus::Completed,
        summary: Some("validated project candidate applied".to_owned()),
        result_kind: Some("project-candidate".to_owned()),
        references: Vec::new(),
        result_handles: vec![
            format!("candidate:{candidate}"),
            "validation:passed".to_owned(),
            format!("snapshot:{successor}"),
        ],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    session
}

/// Verifies public session reads preserve project candidate result handles.
///
/// # Parameters
/// - `client`: Public application client.
/// - `sessions`: Durable session store shared with the application.
/// - `candidate`: Candidate identity returned by the project client.
/// - `successor`: Successor snapshot returned by Apply.
/// - `super`: Parent conformance module.
pub(super) fn assert_public_branch_result(
    client: &Client<InProcessTransport<TestApplication>>,
    sessions: &SqliteSessionStore,
    candidate: &str,
    successor: &str,
) {
    let session = candidate_branch_result(candidate, successor);
    sessions
        .save(&session)
        .expect("persist candidate BranchResult");
    let view = client
        .read_session(session.id.to_string())
        .expect("public session read");
    let branch = view
        .branches
        .iter()
        .find(|branch| branch.title == "PHP implementation")
        .expect("public implementation branch");
    let result = branch.result.as_ref().expect("public BranchResult");
    assert!(
        result
            .result_handles
            .contains(&format!("candidate:{candidate}"))
    );
    assert!(
        result
            .result_handles
            .contains(&"validation:passed".to_owned())
    );
    assert!(
        result
            .result_handles
            .contains(&format!("snapshot:{successor}"))
    );
}

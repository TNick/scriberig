//! Verifies bounded plan-attempt reads through protocol and Rust client.

use super::*;

/// Verifies a plan node with no executions returns an empty safe attempt page.
#[test]
fn remote_client_reads_bounded_attempt_history() {
    let client = scriberig_client::Client::new(scriberig_server::InProcessTransport::new(
        ApplicationService::new(Fixture),
    ));
    let session = "ent_71717171717171717171717171717171";
    let started = client
        .start_agent_with_handles(session, "seed attempt session")
        .expect("seed session");
    let root = started.branch.expect("root branch");
    let plan = client
        .create_plan(SessionPlanCreateRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            anchor: SessionContinuationAnchor::BranchHead { branch: root.id },
            goal: "attempt history plan".to_owned(),
            nodes: vec![SessionPlanNodeDraft {
                key: "work".to_owned(),
                parent_index: None,
                title: "Work".to_owned(),
                purpose: "perform work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            dependencies: Vec::new(),
        })
        .expect("create plan");

    let attempts = client
        .plan_attempts(scriberig_protocol::SessionPlanAttemptsRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id,
            node: plan.nodes[0].id.clone(),
            offset: 0,
            limit: 10,
        })
        .expect("read bounded attempts through client");

    assert_eq!(attempts.total, 0);
    assert_eq!(attempts.attempts, Vec::new());
    assert_eq!(attempts.offset, 0);
    assert_eq!(attempts.next_offset, None);
}

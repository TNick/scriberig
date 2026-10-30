//! Verifies retry history survives semantic plan persistence.

use super::*;
#[path = "plan_attempt_recovery/details.rs"]
mod details;
#[path = "plan_attempt_recovery/revisions.rs"]
mod revisions;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression follows a failed attempt and retry through one durable reload."
)]
/// Retains a failed attempt separately from its later successful retry.
fn semantic_retry_keeps_attempt_outcomes_across_reload() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-semantic-attempt-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([71; 16]);
    let root_id = EntityId::from_bytes([72; 16]);
    let plan_id = EntityId::from_bytes([73; 16]);
    let node_id = EntityId::from_bytes([74; 16]);
    let dependent_id = EntityId::from_bytes([85; 16]);
    let first_branch = EntityId::from_bytes([75; 16]);
    let first_claim = EntityId::from_bytes([76; 16]);
    let failure_id = EntityId::from_bytes([77; 16]);
    let second_branch = EntityId::from_bytes([78; 16]);
    let second_claim = EntityId::from_bytes([79; 16]);
    let success_id = EntityId::from_bytes([80; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let semantic_store =
        scriberig_store_sqlite::SqliteStore::open(&path).expect("open semantic store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "recover failed work",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
        title: "perform task".to_owned(),
        purpose: "complete the requested task".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("add node");
    plan.add_node(PlanNode {
        id: dependent_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
        title: "consume successful result".to_owned(),
        purpose: "run only after the prerequisite succeeds".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("add dependent node");
    plan.add_dependency(node_id, dependent_id)
        .expect("add result dependency");
    let gate_id = EntityId::from_bytes([82; 16]);
    let evidence_id = EntityId::from_bytes([83; 16]);
    plan.add_gate(scriberig_session::PlanGate {
        id: gate_id,
        owner: node_id,
        phase: scriberig_session::PlanGatePhase::Completion,
        requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
            kind: scriberig_session::PlanEvidenceKind::BranchResult,
        },
        waiver: Some(scriberig_session::PlanGateWaiver {
            reason: "test fixture waiver".to_owned(),
            actor: None,
            operation: None,
        }),
    })
    .expect("add gate");
    let entry_gate_id = EntityId::from_bytes([86; 16]);
    plan.add_gate(scriberig_session::PlanGate {
        id: entry_gate_id,
        owner: node_id,
        phase: scriberig_session::PlanGatePhase::Entry,
        requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
            kind: scriberig_session::PlanEvidenceKind::BranchResult,
        },
        waiver: None,
    })
    .expect("add pending entry gate");
    plan.attach_evidence(scriberig_session::PlanEvidenceAssociation {
        id: evidence_id,
        owner: scriberig_session::PlanEvidenceOwner::Gate(gate_id),
        role: scriberig_session::PlanEvidenceRole::Informational,
        evidence: scriberig_session::PlanEvidenceReference {
            kind: scriberig_session::PlanEvidenceKind::Artifact,
            reference: scriberig_session::PlanSemanticReference {
                entity: EntityId::from_bytes([84; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: Some("inspection record".to_owned()),
        actor: None,
    })
    .expect("attach evidence");
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root_id, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan.clone()],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    store.save(&session).expect("save initial plan");
    let mut node_revisions = Vec::with_capacity(7);
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &plan,
        node_id,
        PlanNodeStatus::Planned,
        &mut node_revisions,
    );
    let expected = plan.revision;
    plan.attach_evidence(scriberig_session::PlanEvidenceAssociation {
        id: EntityId::from_bytes([87; 16]),
        owner: scriberig_session::PlanEvidenceOwner::Gate(entry_gate_id),
        role: scriberig_session::PlanEvidenceRole::Supports,
        evidence: scriberig_session::PlanEvidenceReference {
            kind: scriberig_session::PlanEvidenceKind::BranchResult,
            reference: scriberig_session::PlanSemanticReference {
                entity: EntityId::from_bytes([88; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: Some("entry requirement satisfied".to_owned()),
        actor: None,
    })
    .expect("satisfy entry gate");
    session.plans[0] = plan.clone();
    store
        .compare_and_swap_plan(&session, plan_id, expected)
        .expect("persist entry evidence");
    let expected = plan.revision;
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan");
    session.plans[0] = plan.clone();
    store
        .compare_and_swap_plan(&session, plan_id, expected)
        .expect("persist planned lifecycle");
    let expected = plan.revision;
    plan.start_with_coordinator_claim(EntityId::from_bytes([81; 16]), "coordinator", None)
        .expect("start plan");
    session.plans[0] = plan.clone();
    store
        .compare_and_swap_plan(&session, plan_id, expected)
        .expect("persist running lifecycle");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &plan,
        node_id,
        PlanNodeStatus::Ready,
        &mut node_revisions,
    );

    let mut first_attempt = store
        .load(session_id)
        .expect("load initial plan")
        .expect("session exists");
    let expected = first_attempt.plans[0].revision;
    first_attempt.plans[0]
        .start_node(node_id, first_branch, first_claim, "worker", &[])
        .expect("start first attempt");
    let mut first_branch_record = Branch::new(first_branch, "first attempt").expect("branch");
    first_branch_record.parent = Some(root_id);
    first_attempt.branches.push(first_branch_record);
    store
        .compare_and_swap_plan(&first_attempt, plan_id, expected)
        .expect("persist first claim");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &first_attempt.plans[0],
        node_id,
        PlanNodeStatus::Running,
        &mut node_revisions,
    );

    let mut failed_attempt = store
        .load(session_id)
        .expect("load first attempt")
        .expect("session exists");
    let expected = failed_attempt.plans[0].revision;
    let failure = BranchResult {
        id: failure_id,
        branch: first_branch,
        status: BranchStatus::Abandoned,
        summary: Some("first attempt failed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    let failed_branch = failed_attempt
        .branches
        .iter_mut()
        .find(|branch| branch.id == first_branch)
        .expect("first worker branch");
    failed_branch.result = Some(failure.clone());
    failed_branch.status = BranchStatus::Abandoned;
    failed_branch
        .activity
        .push(scriberig_session::ExecutionActivity::BranchResult(
            failure_id,
        ));
    failed_attempt.plans[0]
        .settle_workers(&[], &[(node_id, first_claim, Some(failure))], &[])
        .expect("settle first attempt as failed");
    store
        .compare_and_swap_plan(&failed_attempt, plan_id, expected)
        .expect("persist failed attempt");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &failed_attempt.plans[0],
        node_id,
        PlanNodeStatus::Failed,
        &mut node_revisions,
    );
    assert_eq!(
        failed_attempt.plans[0].dependencies[0].fulfilled_by, None,
        "a failed attempt must not fulfill its dependency"
    );

    let mut retry = store
        .load(session_id)
        .expect("load failed attempt")
        .expect("session exists");
    let expected = retry.plans[0].revision;
    retry.plans[0]
        .retry_failed_node(node_id, &[])
        .expect("retry failed node");
    store
        .compare_and_swap_plan(&retry, plan_id, expected)
        .expect("persist retry");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &retry.plans[0],
        node_id,
        PlanNodeStatus::Ready,
        &mut node_revisions,
    );

    let mut second_attempt = store
        .load(session_id)
        .expect("load retry")
        .expect("session exists");
    let expected = second_attempt.plans[0].revision;
    second_attempt.plans[0]
        .start_node(node_id, second_branch, second_claim, "worker", &[])
        .expect("start second attempt");
    let mut second_branch_record = Branch::new(second_branch, "second attempt").expect("branch");
    second_branch_record.parent = Some(root_id);
    second_attempt.branches.push(second_branch_record);
    store
        .compare_and_swap_plan(&second_attempt, plan_id, expected)
        .expect("persist second attempt");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &second_attempt.plans[0],
        node_id,
        PlanNodeStatus::Running,
        &mut node_revisions,
    );

    let mut completed = store
        .load(session_id)
        .expect("load second attempt")
        .expect("session exists");
    let expected = completed.plans[0].revision;
    let success = BranchResult {
        id: success_id,
        branch: second_branch,
        status: BranchStatus::Completed,
        summary: Some("second attempt succeeded".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    completed.plans[0]
        .settle_workers(
            &[(node_id, second_claim, success.clone())],
            &[],
            std::slice::from_ref(&success),
        )
        .expect("complete second attempt");
    let second_branch_record = completed
        .branches
        .iter_mut()
        .find(|branch| branch.id == second_branch)
        .expect("second worker branch");
    second_branch_record.result = Some(success.clone());
    second_branch_record.status = BranchStatus::Completed;
    second_branch_record
        .activity
        .push(scriberig_session::ExecutionActivity::BranchResult(
            success_id,
        ));
    store
        .compare_and_swap_plan(&completed, plan_id, expected)
        .expect("persist successful retry");
    revisions::observe_node_state(
        &semantic_store,
        session_id,
        plan_id,
        &completed.plans[0],
        node_id,
        PlanNodeStatus::Completed,
        &mut node_revisions,
    );
    assert_eq!(node_revisions.len(), 7);
    assert!(
        node_revisions
            .windows(2)
            .all(|revisions| revisions[0] != revisions[1])
    );

    let reloaded = store
        .load(session_id)
        .expect("reload plan history")
        .expect("session exists");
    details::assert_retry_provenance(
        &reloaded,
        node_id,
        first_branch,
        second_branch,
        failure_id,
        success_id,
    );

    details::AttemptPageFixture {
        store: &store,
        semantic_store: &semantic_store,
        session: session_id,
        plan: plan_id,
        node: node_id,
        gate: gate_id,
        evidence: evidence_id,
        first_branch,
        second_branch,
    }
    .assert_attempt_pages();
    drop(semantic_store);
    drop(store);
    let _ = std::fs::remove_file(path);
}

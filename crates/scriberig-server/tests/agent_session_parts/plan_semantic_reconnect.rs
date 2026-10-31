//! Verifies semantic plan state survives a fresh client connection.

use super::*;
use std::sync::Arc;

/// Retains one dispatcher while clients are replaced to simulate reconnects.
#[derive(Clone)]
struct SharedDispatcherTransport(Arc<Dispatcher<ApplicationService<Fixture>>>);

impl scriberig_protocol::ProtocolTransport for SharedDispatcherTransport {
    /// Sends one request to the shared in-process dispatcher.
    ///
    /// - `request`: Correlated protocol request.
    ///
    /// Correlated protocol response.
    /// # Returns
    /// The dispatcher's correlated response.
    fn send(&self, request: RequestEnvelope) -> scriberig_protocol::ResponseEnvelope {
        self.0.dispatch(request)
    }
}

/// Reloads a gate mutation through a fresh client and authoritative session read.
#[test]
fn client_reconnect_reads_authoritative_semantic_plan_projection() {
    let dispatcher = Arc::new(Dispatcher::new(ApplicationService::new(Fixture)));
    let transport = SharedDispatcherTransport(dispatcher.clone());
    let client = scriberig_client::Client::new(transport);
    let session = "ent_71717171717171717171717171717171";
    let started = client
        .start_agent_with_handles(session, "semantic plan reconnect")
        .expect("start session");
    let root = started.branch.expect("root branch");
    let plan = client
        .create_plan(SessionPlanCreateRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            anchor: SessionContinuationAnchor::BranchHead { branch: root.id },
            goal: "preserve semantic gate state".to_owned(),
            nodes: vec![SessionPlanNodeDraft {
                key: "work".to_owned(),
                parent_index: None,
                title: "Work".to_owned(),
                purpose: "complete required work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            dependencies: Vec::new(),
        })
        .expect("create plan");
    let node = plan.nodes.first().expect("plan node");
    let updated = client
        .mutate_plan(scriberig_protocol::SessionPlanMutationRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            mutation: scriberig_protocol::SessionPlanMutation::AddGate {
                owner: node.id.clone(),
                phase: scriberig_protocol::SessionPlanGatePhase::Entry,
                requirement: scriberig_protocol::SessionPlanGateRequirement::EvidencePresent {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::HumanApproval,
                },
            },
        })
        .expect("add gate through protocol client");

    let reconnected = scriberig_client::Client::new(SharedDispatcherTransport(dispatcher));
    let loaded = reconnected
        .read_session(session)
        .expect("read session after reconnect");

    let reloaded_plan = loaded
        .plans
        .iter()
        .find(|candidate| candidate.id == updated.id)
        .expect("compact plan is available after reconnect");
    assert_eq!(reloaded_plan.exact_revision, updated.exact_revision);
    assert_eq!(
        reloaded_plan.nodes[0].entry_gates.pending, 1,
        "the authoritative compact projection retains the gate summary"
    );
    let details = reconnected
        .plan_details(scriberig_protocol::SessionPlanDetailsRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: updated.id,
            expected_revision: updated.revision,
            snapshot: None,
            gate_offset: 0,
            evidence_offset: 0,
            limit: 10,
        })
        .expect("read canonical gate details after reconnect");
    assert_eq!(details.gate_total, 1);
    assert_eq!(details.gates[0].owner, node.id);
}

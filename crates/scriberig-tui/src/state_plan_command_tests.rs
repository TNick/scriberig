//! Verifies durable plan slash commands reach shared application operations only.
//!
//! Task 150 forbids a plan-local terminal mutation path: a slash command may
//! request a shared `scriberig-app` operation, but it must never write the
//! canonical plan projection that the terminal is showing.

use super::*;

/// Builds one presentation state with a durable plan and an open composer.
///
/// - `draft`: Submitted composer text for the command under test.
///
/// # Returns
/// A state holding one planned plan in one loaded session.
fn state_with_planned_plan(draft: &str) -> TuiState {
    let session_id = scriberig_core::EntityId::from_bytes([171; 16]);
    let branch_id = scriberig_core::EntityId::from_bytes([172; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([173; 16]);
    let mut plan = scriberig_session::Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Route plan work",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: session_id,
            branches: vec![scriberig_session::Branch::new(branch_id, "Root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        conversation_composer: crate::state::ConversationComposerState {
            target: crate::state::ConversationTarget::Existing {
                session: session_id,
                reply_anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: branch_id,
                },
            },
            draft: draft.to_owned(),
            pending_prompt: None,
            pending: false,
            error: None,
        },
        ..TuiState::default()
    }
}

/// Verifies each plan command emits only a shared application request and leaves
/// the canonical plan projection byte-identical, so no plan-local terminal
/// mutation path exists.
#[test]
fn plan_commands_emit_shared_requests_and_never_write_the_durable_projection() {
    for draft in [
        "/plan start",
        "/plan replan widen scope",
        "/plan new verification step",
    ] {
        let mut state = state_with_planned_plan(draft);
        let before = state
            .session_snapshot
            .clone()
            .expect("session snapshot")
            .plans
            .clone();

        let effects = reduce(&mut state, Action::InputSubmitted);

        assert!(
            effects.iter().all(|effect| {
                matches!(
                    effect,
                    Effect::CreatePlan(_)
                        | Effect::Replan(_)
                        | Effect::PlanLifecycle { .. }
                        | Effect::PlanMutation(_)
                        | Effect::SetPlanNodeAssignment(_)
                )
            }),
            "{draft} produced a non-shared effect: {effects:?}"
        );
        assert!(
            !matches!(effects.as_slice(), [Effect::None]),
            "{draft} must reach a shared operation: {effects:?}"
        );
        assert_eq!(
            state
                .session_snapshot
                .clone()
                .expect("session snapshot")
                .plans,
            before,
            "{draft} must not mutate the durable plan projection locally"
        );
    }
}

/// Verifies plan node actions carry selected identities through shared mutation requests.
#[test]
fn plan_node_actions_emit_shared_mutation_requests() {
    let gate = scriberig_core::EntityId::from_bytes([175; 16]);
    let actions = [
        (
            "/plan next Follow up: inspect the result".to_owned(),
            "next",
        ),
        (
            "/plan child Detail: inspect the selected node".to_owned(),
            "child",
        ),
        ("/plan retry".to_owned(), "retry"),
        (format!("/plan waive {gate} reviewed exception"), "waive"),
    ];
    for (draft, expected) in actions {
        let mut state = state_with_planned_plan(&draft);
        let plan = state.session_snapshot.as_ref().unwrap().plans[0].id;
        let selected_node = scriberig_core::EntityId::from_bytes([174; 16]);
        state.plan_cursor = Some(crate::selection::PlanSelection {
            plan,
            node: Some(selected_node),
        });
        let before = state.session_snapshot.as_ref().unwrap().plans.clone();

        let effects = reduce(&mut state, Action::InputSubmitted);

        let [Effect::PlanMutation(request)] = effects.as_slice() else {
            panic!("{expected} must emit one shared mutation request: {effects:?}");
        };
        assert_eq!(request.plan, plan);
        assert_eq!(request.session, state.session_snapshot.as_ref().unwrap().id);
        assert_eq!(request.expected_revision, before[0].revision);
        match (expected, &request.mutation) {
            ("next", scriberig_app::PlanMutation::AddNextStep(node)) => {
                assert_eq!(node.title, "Follow up");
                assert_eq!(node.purpose, "inspect the result");
            }
            (
                "child",
                scriberig_app::PlanMutation::Decompose {
                    parent,
                    children,
                    required: true,
                },
            ) => {
                assert_eq!(*parent, selected_node);
                assert_eq!(children[0].title, "Detail");
            }
            ("retry", scriberig_app::PlanMutation::RetryFailedNode { node }) => {
                assert_eq!(*node, selected_node);
            }
            ("waive", scriberig_app::PlanMutation::WaiveGate { reason, .. }) => {
                assert_eq!(reason, "reviewed exception");
            }
            _ => panic!("unexpected mutation for {expected}: {:?}", request.mutation),
        }
        assert_eq!(state.session_snapshot.as_ref().unwrap().plans, before);
    }
    let mut state =
        state_with_planned_plan("/plan next Add a validation step: run the requested check");
    let plan = state.session_snapshot.as_ref().unwrap().plans[0].id;
    state.plan_cursor = Some(crate::selection::PlanSelection { plan, node: None });
    assert!(matches!(
        reduce(&mut state, Action::InputSubmitted).as_slice(),
        [Effect::PlanMutation(scriberig_app::PlanMutationRequest {
            mutation: scriberig_app::PlanMutation::AddNextStep(_),
            ..
        })]
    ));
}

/// Verifies an evidence inspection command preserves its exact snapshot identity.
#[test]
fn inspect_command_accepts_an_exact_snapshot_reference() {
    let entity = scriberig_core::EntityId::from_bytes([176; 16]);
    let snapshot = scriberig_core::SnapshotId::from_bytes([177; 32]);
    let mut state = state_with_planned_plan(&format!("/inspect {entity} --snapshot {snapshot}"));

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectEntity {
            entity: requested_entity,
            snapshot: Some(requested_snapshot),
            ..
        }] if *requested_entity == entity && *requested_snapshot == snapshot
    ));
}

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

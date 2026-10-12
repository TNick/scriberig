//! Verifies branch runtime state, budgets, cancellation, and safe events.

use scriberig_agent_runtime::orchestration::{
    BranchPhase, BranchRuntimeState, CancellationTree, RuntimeEvent, RuntimeLimits,
};

/// Verifies valid phase transitions and invalid transitions are explicit.
#[test]
fn branch_state_machine_rejects_invalid_terminal_transition() {
    let mut state = BranchRuntimeState::new("branch-1");
    state
        .transition(BranchPhase::CompilingContext)
        .expect("compile");
    state
        .transition(BranchPhase::AwaitingModel)
        .expect("await model");
    state
        .transition(BranchPhase::InterpretingModel)
        .expect("interpret model");
    state.transition(BranchPhase::Completing).expect("complete");
    state.transition(BranchPhase::Completed).expect("completed");

    assert!(state.transition(BranchPhase::CompilingContext).is_err());
}

/// Verifies model/tool budgets fail before another unit of work starts.
#[test]
fn runtime_limits_report_budget_exhaustion() {
    let limits = RuntimeLimits {
        max_model_steps: 1,
        max_tool_calls: 1,
        ..RuntimeLimits::default()
    };
    let mut state = BranchRuntimeState::new("branch-1");
    state.model_steps = 1;

    assert!(limits.check(&state).is_err());
}

/// Verifies root cancellation propagates to a child tree.
#[test]
fn cancellation_propagates_to_children() {
    let root = CancellationTree::new();
    let child = root.child();

    root.cancel();

    assert!(child.is_cancelled());
}

/// Verifies runtime events carry only bounded correlation metadata.
#[test]
fn phase_event_contains_branch_identity() {
    let event = RuntimeEvent::phase_changed("branch-1", BranchPhase::ExecutingTools);

    assert_eq!(event.branch(), "branch-1");
}

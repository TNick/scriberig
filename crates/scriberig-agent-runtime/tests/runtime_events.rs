//! Verifies runtime lifecycle and streaming observations.

use std::sync::atomic::AtomicBool;

use scriberig_agent_runtime::{AgentRuntime, EchoProvider};
use scriberig_app::ApplicationContext;

/// Verifies runtime execution emits bounded lifecycle phase events.
#[test]
fn runtime_emits_phase_events_to_attached_log() {
    let events = scriberig_agent_runtime::orchestration::RuntimeEventLog::new();
    let mut runtime = AgentRuntime::new(EchoProvider, scriberig_agent_runtime::UnconfiguredTool, 1)
        .with_event_log(events.clone())
        .with_branch_id("branch-test");
    runtime
        .run(
            &ApplicationContext::empty(),
            "event",
            &AtomicBool::new(false),
        )
        .expect("runtime result");

    let events = events.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        scriberig_agent_runtime::orchestration::RuntimeEvent::PhaseChanged {
            branch,
            phase: scriberig_agent_runtime::orchestration::BranchPhase::Completed,
        } if branch == "branch-test"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        scriberig_agent_runtime::orchestration::RuntimeEvent::BranchExecutionStarted { branch }
            if branch == "branch-test"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        scriberig_agent_runtime::orchestration::RuntimeEvent::BranchExecutionCompleted { branch }
            if branch == "branch-test"
    )));
}

/// Verifies final provider text is exposed as a bounded runtime delta.
#[test]
fn runtime_emits_model_text_delta_to_attached_log() {
    let events = scriberig_agent_runtime::orchestration::RuntimeEventLog::new();
    let registry = scriberig_app::OperationRegistry::new();
    let mut runtime = AgentRuntime::new(EchoProvider, scriberig_agent_runtime::UnconfiguredTool, 1)
        .with_event_log(events.clone())
        .with_operation_registry(registry.clone())
        .with_branch_id("stream-branch");
    runtime
        .run(
            &ApplicationContext::empty(),
            "streamed answer",
            &AtomicBool::new(false),
        )
        .expect("runtime result");

    assert!(events.snapshot().iter().any(|event| matches!(
        event,
        scriberig_agent_runtime::orchestration::RuntimeEvent::ModelTextDelta {
            branch,
            text
        } if branch == "stream-branch" && text == "streamed answer"
    )));
    assert!(registry.events().iter().any(|event| matches!(
        event,
        scriberig_app::ApplicationEvent::AgentOutputDelta { text, .. }
            if text == "streamed answer"
    )));
}

/// Verifies safe runtime phases are published on the active operation stream.
#[test]
fn runtime_publishes_transient_activity_without_owning_operation_lifecycle() {
    let registry = scriberig_app::OperationRegistry::new();
    let operation = scriberig_app::OperationId::new("observed-agent-1").expect("operation");
    registry
        .start(operation.clone(), scriberig_app::OperationKind::Agent)
        .expect("start operation");
    registry
        .transition(&operation, scriberig_app::OperationStatus::Running)
        .expect("run operation");
    let mut runtime = AgentRuntime::new(EchoProvider, scriberig_agent_runtime::UnconfiguredTool, 1)
        .with_observed_operation(registry.clone(), operation.clone())
        .with_branch_id("branch-live");

    runtime
        .run(
            &ApplicationContext::empty(),
            "live answer",
            &AtomicBool::new(false),
        )
        .expect("runtime result");

    let activity = registry
        .events()
        .into_iter()
        .filter_map(|event| match event {
            scriberig_app::ApplicationEvent::AgentActivity {
                operation: event_operation,
                branch,
                activity_kind,
                state,
                summary,
            } => Some((event_operation, branch, activity_kind, state, summary)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(activity.iter().any(|item| {
        item.0 == operation
            && item.1 == "branch-live"
            && item.2 == "status"
            && item.3 == "running"
            && item.4.contains("AwaitingModel")
    }));
    let activity_sequences = registry
        .events_with_sequences()
        .into_iter()
        .filter_map(|(sequence, event)| {
            matches!(event, scriberig_app::ApplicationEvent::AgentActivity { .. })
                .then_some(sequence)
        })
        .collect::<Vec<_>>();
    assert_ne!(activity_sequences, Vec::<u64>::new());
    assert!(activity_sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        registry.get(&operation).expect("operation state").status,
        scriberig_app::OperationStatus::Running
    );
}

/// Verifies runtime text fragments reach an attached presentation callback.
#[test]
fn runtime_forwards_model_text_to_callback() {
    let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let target = observed.clone();
    let callback = std::sync::Arc::new(move |text: &str| {
        target.lock().expect("callback lock").push(text.to_owned());
    });
    let mut runtime = AgentRuntime::new(EchoProvider, scriberig_agent_runtime::UnconfiguredTool, 1)
        .with_output_callback(callback);
    runtime
        .run(
            &ApplicationContext::empty(),
            "callback answer",
            &AtomicBool::new(false),
        )
        .expect("runtime result");

    assert_eq!(
        observed.lock().expect("result lock").as_slice(),
        ["callback answer"]
    );
}

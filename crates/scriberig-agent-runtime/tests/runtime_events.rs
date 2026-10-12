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

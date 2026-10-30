//! Covers terminal runtime composition and bounded work scheduling.

use scriberig_app::{ApplicationContext, ApplicationError, InspectResult};

use crate::TuiService;
use crate::{action::Action, state::TuiState};

/// Provides an unsupported operation for runtime composition tests.
struct Fixture;

impl crate::service::TuiModelRequestService for Fixture {}

impl TuiService for Fixture {
    /// Rejects inspection while keeping the runtime test deterministic.
    ///
    /// - `_context`: Test application context.
    /// - `_entity`: Test entity identity.
    /// - `_snapshot`: Optional test snapshot.
    ///
    /// Never returns a successful result.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect_entity(
        &self,
        _context: &ApplicationContext,
        _entity: scriberig_core::EntityId,
        _snapshot: Option<scriberig_core::SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test".to_owned(),
        })
    }
}

/// Blocks a new-session agent effect until the test releases it.
struct BlockingAgentFixture {
    /// Signals when the accepted agent execution begins.
    started: std::sync::mpsc::Sender<()>,
    /// Holds the release signal until the test verifies worker availability.
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

/// Blocks plan lifecycle execution until the test releases it.
struct BlockingPlanFixture {
    /// Signals when plan execution begins.
    started: std::sync::mpsc::Sender<()>,
    /// Holds the release signal until the test checks worker availability.
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl crate::service::TuiModelRequestService for BlockingPlanFixture {}

impl TuiService for BlockingPlanFixture {
    /// Rejects inspection requests in the plan scheduling test.
    ///
    /// - `self`: Test fixture.
    /// - `_context`: Test application context.
    /// - `_entity`: Requested entity.
    /// - `_snapshot`: Optional snapshot.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// Never returns a successful inspection.
    /// # Errors
    /// Returns an error because inspection is unsupported.
    fn inspect_entity(
        &self,
        _context: &ApplicationContext,
        _entity: scriberig_core::EntityId,
        _snapshot: Option<scriberig_core::SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test inspection".to_owned(),
        })
    }

    /// Waits for release to prove plan work does not occupy the effect worker.
    ///
    /// - `self`: Gate controlled by the concurrency test.
    /// - `_context`: Test application context.
    /// - `_request`: Plan lifecycle request.
    /// - `_action`: Requested plan transition.
    ///
    /// A valid empty test plan after release.
    /// # Returns
    /// The test plan.
    /// # Errors
    /// Returns an error when the release signal closes.
    fn plan_lifecycle(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::PlanLifecycleRequest,
        _action: scriberig_app::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.started.send(()).expect("notify plan start");
        self.release
            .lock()
            .expect("release lock")
            .recv()
            .expect("release plan execution");
        scriberig_session::Plan::new(
            scriberig_core::EntityId::from_bytes([11; 16]),
            scriberig_core::EntityId::from_bytes([12; 16]),
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: scriberig_core::EntityId::from_bytes([13; 16]),
            },
            "test plan",
        )
        .map_err(|error| ApplicationError::Internal {
            message: format!("test plan construction failed: {error:?}"),
        })
    }
}

impl crate::service::TuiModelRequestService for BlockingAgentFixture {}

impl TuiService for BlockingAgentFixture {
    /// Rejects unrelated inspection requests in the concurrency test.
    ///
    /// - `_context`: Test application context.
    /// - `_entity`: Test entity identity.
    /// - `_snapshot`: Optional exact snapshot.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when inspection is not supported.
    fn inspect_entity(
        &self,
        _context: &ApplicationContext,
        _entity: scriberig_core::EntityId,
        _snapshot: Option<scriberig_core::SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test inspection".to_owned(),
        })
    }

    /// Waits for release to prove unrelated effects can run during agent work.
    ///
    /// - `self`: Gate controlled by the concurrency test.
    /// - `_context`: Test application context.
    /// - `_prompt`: Submitted deterministic prompt.
    /// - `_selection`: Optional semantic selection.
    ///
    /// Stable handles for the accepted test execution.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the test release channel closes.
    fn start_new_session_handle(
        &self,
        _context: &ApplicationContext,
        _prompt: &str,
        _selection: Option<scriberig_app::AgentSelection>,
    ) -> Result<crate::service::AgentHandle, ApplicationError> {
        self.started.send(()).expect("notify start");
        self.release
            .lock()
            .expect("release lock")
            .recv()
            .expect("release agent execution");
        Ok(crate::service::AgentHandle {
            operation: scriberig_core::OperationId::new("agent-test").expect("operation id"),
            session: scriberig_core::EntityId::from_bytes([1; 16]),
            branch: Some(scriberig_core::EntityId::from_bytes([2; 16])),
            workspace: None,
            failure: None,
        })
    }
}

/// Verifies a runtime quit action is handled without a terminal backend.
#[test]
fn quit_action_terminates_state_loop() {
    let mut state = TuiState::default();
    crate::state::reduce(&mut state, Action::QuitRequested);
    assert!(state.should_quit);
}

/// Verifies non-interactive startup is rejected before terminal takeover.
#[test]
fn rejects_noninteractive_terminal_for_interactive_runtime() {
    let error = super::validate_interactive_terminal(true, false, true)
        .expect_err("non-terminal input must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::NotConnected);
}

/// Verifies headless composition does not require an attached terminal.
#[test]
fn accepts_noninteractive_composition_without_terminal() {
    super::validate_interactive_terminal(false, false, false)
        .expect("headless composition must remain usable");
}

/// Verifies headless startup does not enter terminal mode.
#[test]
fn noninteractive_run_does_not_take_over_terminal() {
    let result = super::run(
        &Fixture,
        &ApplicationContext::empty(),
        &crate::TuiConfig::default(),
    );
    assert!(result.is_ok());
}

/// Verifies semantic effects are routed through the worker boundary.
#[test]
fn worker_routes_effect_results_back_to_runtime() {
    let (work_sender, work_receiver) = std::sync::mpsc::sync_channel(1);
    let (message_sender, message_receiver) = std::sync::mpsc::sync_channel(1);
    let context = ApplicationContext::empty();
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            super::worker_loop(scope, &Fixture, context, work_receiver, message_sender);
        });
        work_sender
            .send(super::Work::Effect(Box::new(
                crate::effect::Effect::RequestCapabilities,
            )))
            .expect("worker accepts effect");
        drop(work_sender);
        let message = message_receiver.recv().expect("worker result");
        assert!(matches!(message, super::RuntimeMessage::Action(_)));
        worker.join().expect("worker exits");
    });
}

/// Verifies a blocked new conversation does not starve other service effects.
#[test]
fn blocked_new_conversation_keeps_worker_available() {
    let (started_sender, started_receiver) = std::sync::mpsc::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let service = BlockingAgentFixture {
        started: started_sender,
        release: std::sync::Mutex::new(release_receiver),
    };
    let (work_sender, work_receiver) = std::sync::mpsc::sync_channel(2);
    let (message_sender, message_receiver) = std::sync::mpsc::sync_channel(2);
    let context = ApplicationContext::empty();
    let service_ref = &service;
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            super::worker_loop(scope, service_ref, context, work_receiver, message_sender);
        });
        work_sender
            .send(super::Work::Effect(Box::new(
                crate::effect::Effect::StartNewConversation {
                    prompt: "test agent".to_owned(),
                    selection: None,
                },
            )))
            .expect("queue new conversation");
        started_receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("agent execution starts");
        work_sender
            .send(super::Work::Effect(Box::new(
                crate::effect::Effect::RequestCapabilities,
            )))
            .expect("queue independent effect");
        assert!(matches!(
            message_receiver.recv_timeout(std::time::Duration::from_secs(1)),
            Ok(super::RuntimeMessage::Action(_))
        ));
        release_sender.send(()).expect("release agent");
        assert!(matches!(
            message_receiver.recv_timeout(std::time::Duration::from_secs(1)),
            Ok(super::RuntimeMessage::Action(_))
        ));
        drop(work_sender);
        worker.join().expect("worker exits");
    });
}

/// Verifies a blocking coordinator does not starve local slash commands.
#[test]
fn blocked_plan_lifecycle_keeps_worker_available() {
    let (started_sender, started_receiver) = std::sync::mpsc::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let service = BlockingPlanFixture {
        started: started_sender,
        release: std::sync::Mutex::new(release_receiver),
    };
    let (work_sender, work_receiver) = std::sync::mpsc::sync_channel(2);
    let (message_sender, message_receiver) = std::sync::mpsc::sync_channel(2);
    let context = ApplicationContext::empty();
    let service_ref = &service;
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            super::worker_loop(scope, service_ref, context, work_receiver, message_sender);
        });
        work_sender
            .send(super::Work::Effect(Box::new(
                crate::effect::Effect::PlanLifecycle {
                    request: scriberig_app::PlanLifecycleRequest {
                        session: scriberig_core::EntityId::from_bytes([12; 16]),
                        plan: scriberig_core::EntityId::from_bytes([11; 16]),
                        expected_revision: 1,
                    },
                    action: scriberig_app::PlanLifecycleAction::Start,
                },
            )))
            .expect("queue plan lifecycle");
        started_receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("plan execution starts");
        work_sender
            .send(super::Work::Effect(Box::new(
                crate::effect::Effect::RequestCapabilities,
            )))
            .expect("queue independent effect");
        assert!(matches!(
            message_receiver.recv_timeout(std::time::Duration::from_secs(1)),
            Ok(super::RuntimeMessage::Action(_))
        ));
        release_sender.send(()).expect("release plan execution");
        assert!(matches!(
            message_receiver.recv_timeout(std::time::Duration::from_secs(1)),
            Ok(super::RuntimeMessage::Action(_))
        ));
        drop(work_sender);
        worker.join().expect("worker exits");
    });
}

/// Verifies no-op effects are dropped and real effects remain ordered.
#[test]
fn full_worker_queue_skips_noop_effects_and_retains_real_work() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    sender
        .send(super::Work::PollApplicationEvents)
        .expect("occupy worker queue");
    let mut pending = std::collections::VecDeque::new();

    super::queue_effects(
        &sender,
        &mut pending,
        vec![
            crate::effect::Effect::None,
            crate::effect::Effect::RequestCapabilities,
        ],
    );

    assert_eq!(pending.len(), 1);
    assert!(matches!(
        receiver.recv().expect("poll work"),
        super::Work::PollApplicationEvents
    ));
    super::flush_effects(&sender, &mut pending);
    assert!(matches!(
        receiver.recv().expect("real effect"),
        super::Work::Effect(effect)
            if *effect == crate::effect::Effect::RequestCapabilities
    ));
    assert!(pending.is_empty());
}

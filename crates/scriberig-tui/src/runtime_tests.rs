//! Covers terminal runtime composition and bounded work scheduling.

use scriberig_app::{ApplicationContext, ApplicationError, InspectResult};

use crate::TuiService;
use crate::{
    event::{Key, KeyCode, Modifiers, TuiEvent},
    state::TuiState,
};

/// Provides an unsupported operation for runtime composition tests.
struct Fixture;

impl TuiService for Fixture {
    /// Rejects inspection while keeping the runtime test deterministic.
    ///
    /// # Parameters
    /// - `_context`: Test application context.
    /// - `_entity`: Test entity identity.
    /// - `_snapshot`: Optional test snapshot.
    ///
    /// # Returns
    /// Never returns a successful result.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
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

/// Verifies a runtime quit event is handled without a terminal backend.
#[test]
fn quit_event_terminates_state_loop() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
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
            .send(super::Work::Effect(
                crate::effect::Effect::RequestCapabilities,
            ))
            .expect("worker accepts effect");
        drop(work_sender);
        let message = message_receiver.recv().expect("worker result");
        assert!(matches!(message, super::RuntimeMessage::Action(_)));
        worker.join().expect("worker exits");
    });
}

/// Verifies reducer effects remain pending when the bounded worker queue is full.
#[test]
fn full_worker_queue_retains_effects_in_order() {
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

    assert_eq!(pending.len(), 2);
    assert!(matches!(
        receiver.recv().expect("poll work"),
        super::Work::PollApplicationEvents
    ));
    super::flush_effects(&sender, &mut pending);
    assert!(matches!(
        receiver.recv().expect("first effect"),
        super::Work::Effect(crate::effect::Effect::None)
    ));
    super::flush_effects(&sender, &mut pending);
    assert!(matches!(
        receiver.recv().expect("second effect"),
        super::Work::Effect(crate::effect::Effect::RequestCapabilities)
    ));
    assert!(pending.is_empty());
}

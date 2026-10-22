//! Verifies concurrent console clients cannot overwrite newer selections.

use rpi_ai::{Api, Model, ThinkingLevel, ThinkingLevelMap};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService,
};
use scriberig_protocol::{ProtocolVersion, Request, RequestEnvelope, Response, SessionReference};
use scriberig_server::Dispatcher;

/// Backend fixture for application-owned session selection behavior.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations unused by this protocol integration test.
    ///
    /// - `_context`: Trusted request context.
    /// - `_request`: Universal operation request.
    ///
    /// Never returns a successful universal operation.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "concurrency fixture operation".to_owned(),
        })
    }
}

/// Creates a correlated request envelope for one simulated client.
///
/// - `correlation_id`: Client request identity.
/// - `request`: Typed public protocol request.
///
/// A request using the current protocol version.
/// # Returns
/// The operation result.
fn envelope(correlation_id: &str, request: Request) -> RequestEnvelope {
    RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: correlation_id.to_owned(),
        request,
    }
}

/// Verifies stale model and thinking writes conflict across two clients.
#[test]
fn stale_model_and_thinking_writes_conflict_between_clients() {
    let service = ApplicationService::new(Fixture).with_upstream_models(vec![
        Model::new(
            "wire-a",
            "Model A",
            Api::AnthropicMessages,
            "provider-a",
            "https://example.test",
        ),
        model_b(),
    ]);
    let tui_client = Dispatcher::new(&service);
    let web_client = Dispatcher::new(&service);
    let session = SessionReference {
        id: "ent_01010101010101010101010101010101".to_owned(),
    };

    let selected = tui_client.dispatch(envelope(
        "first-model",
        Request::SessionModelSelection(scriberig_protocol::SessionModelSelectionRequest {
            session: session.clone(),
            expected_revision: 0,
            provider_id: "provider-a".to_owned(),
            model_id: "wire-a".to_owned(),
        }),
    ));
    let Response::SessionExecution(selected) = selected.response else {
        panic!("unexpected model selection response: {selected:?}");
    };
    assert_eq!(selected.revision, 1);

    let observed_by_web = web_client.dispatch(envelope(
        "web-observes-model-a",
        Request::SessionRead(scriberig_protocol::SessionReadRequest {
            session: session.clone(),
            branch: None,
        }),
    ));
    let Response::Session(observed_by_web) = observed_by_web.response else {
        panic!("unexpected session read response: {observed_by_web:?}");
    };
    assert_eq!(observed_by_web.execution, selected);

    let changed = tui_client.dispatch(envelope(
        "first-thinking",
        Request::SessionThinkingSelection(scriberig_protocol::SessionThinkingSelectionRequest {
            session: session.clone(),
            expected_revision: selected.revision,
            thinking_level: "high".to_owned(),
        }),
    ));
    let Response::SessionExecution(changed) = changed.response else {
        panic!("unexpected thinking selection response: {changed:?}");
    };
    assert_eq!(changed.revision, 2);

    let stale_thinking = web_client.dispatch(envelope(
        "second-stale-thinking",
        Request::SessionThinkingSelection(scriberig_protocol::SessionThinkingSelectionRequest {
            session: session.clone(),
            expected_revision: selected.revision,
            thinking_level: "medium".to_owned(),
        }),
    ));
    assert!(matches!(stale_thinking.response, Response::Error(_)));

    let switched = tui_client.dispatch(envelope(
        "first-switch-model",
        Request::SessionModelSelection(scriberig_protocol::SessionModelSelectionRequest {
            session: session.clone(),
            expected_revision: changed.revision,
            provider_id: "provider-b".to_owned(),
            model_id: "wire-b".to_owned(),
        }),
    ));
    let Response::SessionExecution(switched) = switched.response else {
        panic!("unexpected switched model response: {switched:?}");
    };
    assert_eq!(switched.revision, 3);
    assert_eq!(switched.model_id.as_deref(), Some("wire-b"));
    assert_eq!(switched.thinking_level.as_deref(), Some("low"));

    let observed_by_web = web_client.dispatch(envelope(
        "web-observes-model-b",
        Request::SessionRead(scriberig_protocol::SessionReadRequest {
            session: session.clone(),
            branch: None,
        }),
    ));
    let Response::Session(observed_by_web) = observed_by_web.response else {
        panic!("unexpected session read response: {observed_by_web:?}");
    };
    assert_eq!(observed_by_web.execution, switched);

    assert_web_model_write_and_stale_rejection(
        &tui_client,
        &web_client,
        session,
        &switched,
        &changed,
    );
}

/// Creates the upstream model whose low-only support clamps earlier choices.
///
/// The provider-b model used by selection tests.
/// # Returns
/// The operation result.
fn model_b() -> Model {
    let mut model = Model::new(
        "wire-b",
        "Model B",
        Api::AnthropicMessages,
        "provider-b",
        "https://example.test",
    );
    model.thinking_level_map = Some(ThinkingLevelMap::from([
        (ThinkingLevel::Minimal, None),
        (ThinkingLevel::Low, Some("low".to_owned())),
        (ThinkingLevel::Medium, None),
        (ThinkingLevel::High, None),
    ]));
    model
}

/// Applies the second client's update and verifies stale writes are rejected.
///
/// - `tui_client`: First protocol dispatcher.
/// - `web_client`: Second protocol dispatcher.
/// - `session`: Shared session identity.
/// - `switched`: Latest model selection observed by both clients.
/// - `changed`: Earlier revision used to make a stale write.
fn assert_web_model_write_and_stale_rejection(
    tui_client: &Dispatcher<&ApplicationService<Fixture>>,
    web_client: &Dispatcher<&ApplicationService<Fixture>>,
    session: SessionReference,
    switched: &scriberig_protocol::SessionExecutionSelection,
    changed: &scriberig_protocol::SessionExecutionSelection,
) {
    let selected_by_web = web_client.dispatch(envelope(
        "web-selects-model-a",
        Request::SessionModelSelection(scriberig_protocol::SessionModelSelectionRequest {
            session: session.clone(),
            expected_revision: switched.revision,
            provider_id: "provider-a".to_owned(),
            model_id: "wire-a".to_owned(),
        }),
    ));
    let Response::SessionExecution(selected_by_web) = selected_by_web.response else {
        panic!("unexpected model selection response: {selected_by_web:?}");
    };
    assert_eq!(selected_by_web.revision, 4);

    let observed_by_tui = tui_client.dispatch(envelope(
        "tui-observes-web-model-a",
        Request::SessionRead(scriberig_protocol::SessionReadRequest {
            session: session.clone(),
            branch: None,
        }),
    ));
    let Response::Session(observed_by_tui) = observed_by_tui.response else {
        panic!("unexpected session read response: {observed_by_tui:?}");
    };
    assert_eq!(observed_by_tui.execution, selected_by_web);

    let stale_model = web_client.dispatch(envelope(
        "second-stale-model",
        Request::SessionModelSelection(scriberig_protocol::SessionModelSelectionRequest {
            session,
            expected_revision: changed.revision,
            provider_id: "provider-a".to_owned(),
            model_id: "wire-a".to_owned(),
        }),
    ));
    assert!(matches!(stale_model.response, Response::Error(_)));
}

//! Verifies upstream provider failures keep their public `ScribeRig` categories.

use std::sync::Arc;

use async_trait::async_trait;
use rpi_ai::event_stream::create_assistant_message_event_stream;
use rpi_ai::providers::faux::{FauxScript, FauxStep};
use rpi_ai::types::{Api, AssistantMessage, AssistantMessageEvent, ErrorReason, StopReason};
use rpi_ai::{Context, Model, Provider, SimpleStreamOptions};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchError, UpstreamBranchInvocation, run_upstream_branch_invocation,
    run_upstream_branch_invocation_blocking,
};
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

/// One upstream terminal outcome used by the fixture provider.
enum FixtureOutcome {
    /// Upstream classified the request as an authentication or authorization fault.
    Authentication,
    /// Upstream reported an ordinary transport/provider fault.
    Transport,
    /// Upstream aborted the request on its own.
    Aborted,
    /// Upstream aborted only after the host cancellation token fired.
    AbortedOnCancel,
}

/// Deterministic upstream provider emitting one terminal classification.
struct FixtureProvider {
    /// Stable fixture provider identity.
    provider_id: String,
    /// Exact model served by the fixture.
    served: Vec<Model>,
    /// Terminal outcome the fixture produces.
    outcome: FixtureOutcome,
}

#[async_trait]
impl Provider for FixtureProvider {
    /// Returns the fixture provider identity.
    ///
    /// # Returns
    /// Stable fixture provider identifier.
    fn id(&self) -> &str {
        &self.provider_id
    }

    /// Returns the models served by the fixture.
    ///
    /// # Returns
    /// Exact fixture model rows.
    fn models(&self) -> &[Model] {
        &self.served
    }

    /// Streams one terminal assistant event for the configured outcome.
    ///
    /// - `self`: Fixture provider holding the configured outcome.
    /// - `model`: Exact model selected by the upstream loop.
    /// - `_ctx`: Provider context, unused by the fixture.
    /// - `opts`: Stream options carrying the host cancellation signal.
    ///
    /// # Returns
    /// The terminal assistant event stream.
    async fn stream_simple(
        &self,
        model: &Model,
        _ctx: &Context,
        opts: &SimpleStreamOptions,
    ) -> rpi_ai::AssistantMessageEventStream {
        let (mut producer, stream) = create_assistant_message_event_stream();
        let provider_id = self.provider_id.clone();
        let model_id = model.id.clone();
        let api = model.api.clone();
        let signal = opts.signal.clone();
        let authentication = matches!(self.outcome, FixtureOutcome::Authentication);
        let transport = matches!(self.outcome, FixtureOutcome::Transport);
        let wait_for_cancel = matches!(self.outcome, FixtureOutcome::AbortedOnCancel);
        tokio::spawn(async move {
            let (reason, stop_reason, message) = if authentication {
                (
                    ErrorReason::Authentication,
                    StopReason::Error,
                    "provider authentication failed",
                )
            } else if transport {
                (
                    ErrorReason::Error,
                    StopReason::Error,
                    "provider transport unavailable",
                )
            } else {
                if wait_for_cancel {
                    signal.cancelled().await;
                }
                (
                    ErrorReason::Aborted,
                    StopReason::Aborted,
                    "provider request cancelled",
                )
            };
            producer.push(AssistantMessageEvent::Error {
                reason,
                error: terminal(api, provider_id, model_id, stop_reason, message),
            });
        });
        stream
    }
}

/// Builds one terminal assistant message for a fixture outcome.
///
/// # Parameters
/// - `api`: API identity of the served model.
/// - `provider`: Provider identity that produced the response.
/// - `model`: Model identity that produced the response.
/// - `stop_reason`: Terminal stop reason.
/// - `message`: Safe terminal failure text.
///
/// # Returns
/// The upstream terminal assistant message.
fn terminal(
    api: Api,
    provider: String,
    model: String,
    stop_reason: StopReason,
    message: &str,
) -> AssistantMessage {
    AssistantMessage::terminal(api, provider, model, stop_reason, message, 0)
}

/// Builds one compiled branch invocation for the fixture provider.
///
/// # Parameters
/// - `provider`: Upstream fixture provider.
/// - `model`: Exact model bound to the invocation.
///
/// # Returns
/// A branch invocation with no semantic tools.
fn invocation(provider: Arc<dyn Provider>, model: Model) -> UpstreamBranchInvocation {
    let context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "session",
            "branch",
            "provider failure proof",
            "review the failing provider",
        ));
    UpstreamBranchInvocation {
        branch: EntityId::from_bytes([71; 16]),
        provider,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: None,
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
    }
}

/// Builds one fixture model row.
///
/// # Parameters
/// - `provider_id`: Provider identity owning the model.
/// - `model_id`: Model identity served by the fixture.
///
/// # Returns
/// An upstream model descriptor bound to the fixture provider.
fn fixture_model(provider_id: &str, model_id: &str) -> Model {
    Model::new(
        model_id,
        "Fixture Model",
        Api::Other("fixture".to_owned()),
        provider_id,
        "https://provider.invalid",
    )
}

/// Runs one fixture branch invocation to completion on the current-thread runtime.
///
/// # Parameters
/// - `provider`: Upstream fixture provider.
/// - `model`: Exact model bound to the invocation.
///
/// # Returns
/// The branch invocation outcome.
///
/// # Errors
/// Returns the branch failure produced by the upstream outcome.
fn run(
    provider: Arc<dyn Provider>,
    model: Model,
) -> Result<(), scriberig_agent_runtime::upstream_invocation::UpstreamBranchFailure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("invocation runtime");
    runtime
        .block_on(run_upstream_branch_invocation(
            invocation(provider, model),
            "review the failing provider",
            CancellationToken::new(),
            None,
        ))
        .map(|_| ())
}

/// Keeps an upstream authentication failure in the public authentication category.
#[test]
fn authentication_failure_maps_to_the_authentication_runtime_category() {
    let provider_id = "fixture-auth";
    let model = fixture_model(provider_id, "auth-model");
    let provider = Arc::new(FixtureProvider {
        provider_id: provider_id.to_owned(),
        served: vec![model.clone()],
        outcome: FixtureOutcome::Authentication,
    });

    let failure = run(Arc::clone(&provider) as Arc<dyn Provider>, model).expect_err("auth failure");
    assert_eq!(
        failure.error,
        UpstreamBranchError::Authentication("provider authentication failed".to_owned())
    );
    assert_eq!(failure.model_requests.len(), 1);
    let public = scriberig_agent_runtime::AgentError::from(failure.error);
    assert!(public.is_authentication());
    assert_eq!(
        public,
        scriberig_agent_runtime::AgentError::Authentication(
            "provider authentication failed".to_owned()
        )
    );
}

/// Keeps an upstream transport failure out of the authentication category.
#[test]
fn transport_failure_maps_to_the_provider_runtime_category() {
    let provider_id = "fixture-transport";
    let model = fixture_model(provider_id, "transport-model");
    let provider = Arc::new(FixtureProvider {
        provider_id: provider_id.to_owned(),
        served: vec![model.clone()],
        outcome: FixtureOutcome::Transport,
    });

    let failure =
        run(Arc::clone(&provider) as Arc<dyn Provider>, model).expect_err("transport failure");
    assert_eq!(
        failure.error,
        UpstreamBranchError::Agent("provider transport unavailable".to_owned())
    );
    assert!(!matches!(
        failure.error,
        UpstreamBranchError::Authentication(_) | UpstreamBranchError::Cancelled
    ));
    assert_eq!(failure.model_requests.len(), 1);
    assert_eq!(
        scriberig_agent_runtime::AgentError::from(failure.error),
        scriberig_agent_runtime::AgentError::Provider(
            "upstream agent: provider transport unavailable".to_owned()
        )
    );
}

/// Reports an upstream-side abort as `ScribeRig` cancellation, not a provider fault.
#[test]
fn provider_abort_maps_to_the_cancellation_runtime_category() {
    let provider_id = "fixture-abort";
    let model = fixture_model(provider_id, "abort-model");
    let provider = Arc::new(FixtureProvider {
        provider_id: provider_id.to_owned(),
        served: vec![model.clone()],
        outcome: FixtureOutcome::Aborted,
    });

    let failure =
        run(Arc::clone(&provider) as Arc<dyn Provider>, model).expect_err("aborted invocation");
    assert_eq!(failure.error, UpstreamBranchError::Cancelled);
    assert_eq!(
        scriberig_agent_runtime::AgentError::from(failure.error),
        scriberig_agent_runtime::AgentError::Cancelled
    );
}

/// Cancels a pending upstream request through the branch-local `ScribeRig` token.
#[test]
fn branch_cancellation_of_a_pending_request_reports_cancellation() {
    let provider_id = "fixture-pending";
    let model = fixture_model(provider_id, "pending-model");
    let provider = Arc::new(FixtureProvider {
        provider_id: provider_id.to_owned(),
        served: vec![model.clone()],
        outcome: FixtureOutcome::AbortedOnCancel,
    });
    let cancellation = CancellationToken::new();
    cancellation.clone().cancel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("invocation runtime");
    let failure = runtime
        .block_on(run_upstream_branch_invocation(
            invocation(Arc::clone(&provider) as Arc<dyn Provider>, model.clone()),
            "review the failing provider",
            cancellation,
            None,
        ))
        .expect_err("cancelled invocation");
    assert_eq!(failure.error, UpstreamBranchError::Cancelled);
    assert_eq!(
        scriberig_agent_runtime::AgentError::from(failure.error),
        scriberig_agent_runtime::AgentError::Cancelled
    );
}

/// Keeps an exhausted upstream faux script in the provider category with its request.
#[test]
fn faux_provider_failure_retains_the_prepared_request_projection() {
    let provider = FauxScript::new();
    provider.set_responses(vec![FauxStep::Message(AssistantMessage::terminal(
        Api::Faux,
        "faux",
        "faux-1",
        StopReason::Error,
        "upstream scenario exhausted",
        0,
    ))]);
    let provider = rpi_ai::providers::faux::FauxProvider::with_models(
        provider,
        vec![fixture_model("faux", "faux-1")],
    );
    let model = provider.default_model().clone();

    let failure = run_upstream_branch_invocation_blocking(
        invocation(provider as Arc<dyn Provider>, model),
        "review the failing provider",
    )
    .expect_err("faux failure");
    assert_eq!(
        failure.error,
        UpstreamBranchError::Agent("upstream scenario exhausted".to_owned())
    );
    assert_eq!(failure.model_requests.len(), 1);
    assert!(
        failure.model_requests[0]
            .messages
            .iter()
            .any(|message| message.content.contains("review the failing provider"))
    );
}

/// Keeps structural branch failures in the public provider runtime category.
///
/// The product execution adapters used to spell this projection out inline. The
/// shared `From` boundary must keep the same categories so a mis-scoped branch
/// tool set or a missing assistant turn is never presented as an authentication
/// failure or a cancellation.
#[test]
fn structural_branch_failures_project_to_the_provider_runtime_category() {
    for error in [
        UpstreamBranchError::InvalidToolSet,
        UpstreamBranchError::MissingFinalAssistant,
    ] {
        let public = scriberig_agent_runtime::AgentError::from(error.clone());
        assert_eq!(
            public,
            scriberig_agent_runtime::AgentError::Provider(error.to_string())
        );
        assert!(!public.is_authentication());
    }
}

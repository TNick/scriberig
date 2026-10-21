//! Binds upstream provider streaming to a `ScribeRig` branch invocation.

use std::sync::Arc;

use rpi_ai::{AppliedRequestSettings, AppliedRequestSettingsCallback, AssistantMessageEvent};

use super::request_projection;
use super::{
    CapturedRequestSink, UpstreamBranchInvocation, applied_settings::capture_applied_settings,
};

/// Builds the upstream callback that validates and records each provider request.
///
/// # Parameters
/// - `invocation`: Exact branch authorization and request configuration.
/// - `captured_requests`: Sink for bounded request provenance.
///
/// # Returns
/// Cancellable upstream stream callback bound to the authorized model.
pub(super) fn build_provider_stream(
    invocation: &UpstreamBranchInvocation,
    captured_requests: CapturedRequestSink,
) -> rpi_agent::CancellableContinuationStreamFn {
    let model = invocation.model.clone();
    let provider = Arc::clone(&invocation.provider);
    let request_secret = invocation.request_secret.clone();
    let bearer = invocation.bearer;
    let applied_settings = invocation.applied_settings.clone();
    let compiled_context = invocation.context.clone();
    let provider_profile = invocation.provider_profile.clone();
    let thinking_level = invocation.thinking_level;
    let continuation_enabled = provider.supports_response_continuation();
    Arc::new(
        move |requested_model, context, options, continuation, cancellation| {
            if requested_model.provider != model.provider
                || requested_model.id != model.id
                || requested_model.api != model.api
            {
                let (mut producer, stream) =
                    rpi_ai::event_stream::create_assistant_message_event_stream();
                producer.push(AssistantMessageEvent::Error {
                    reason: rpi_ai::types::ErrorReason::Error,
                    error: rpi_ai::types::AssistantMessage::terminal(
                        model.api.clone(),
                        provider.id(),
                        model.id.clone(),
                        rpi_ai::types::StopReason::Error,
                        "upstream requested a model outside the bound branch invocation",
                        0,
                    ),
                });
                return stream;
            }
            let projection = request_projection::capture_provider_request(
                &model,
                context,
                provider_profile.as_deref(),
                &compiled_context,
                thinking_level,
            );
            let request_index = if let Ok(mut requests) = captured_requests.lock() {
                let mut projection = projection;
                projection.ordinal = u64::try_from(requests.len()).unwrap_or(u64::MAX);
                requests.push(projection);
                Some(requests.len() - 1)
            } else {
                None
            };
            let captured_settings = Arc::clone(&captured_requests);
            let expected_provider = model.provider.clone();
            let expected_model = model.id.clone();
            let expected_api = model.api.as_str().to_owned();
            let forwarded_settings = applied_settings.clone();
            let callback: AppliedRequestSettingsCallback =
                Arc::new(move |settings: AppliedRequestSettings| {
                    capture_applied_settings(
                        settings,
                        request_index,
                        &captured_settings,
                        &expected_provider,
                        &expected_model,
                        &expected_api,
                        forwarded_settings.as_ref(),
                    );
                });
            rpi_ai::stream_provider_cancellable(
                Arc::clone(&provider),
                model.clone(),
                context.clone(),
                options.clone(),
                request_secret.clone().map(|secret| (secret, bearer)),
                Some(callback),
                continuation.cloned(),
                continuation_enabled,
                cancellation,
            )
        },
    )
}

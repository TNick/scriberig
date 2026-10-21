//! Captures the upstream provider's safe applied-settings report.

use rpi_ai::{AppliedRequestSettings, AppliedRequestSettingsCallback};

use super::CapturedRequestSink;

/// Records an upstream applied-settings report after validating model identity.
///
/// # Parameters
/// - `settings`: Upstream report about transformed provider request settings.
/// - `request_index`: Captured request updated by the report.
/// - `captured_requests`: Sink for bounded request provenance.
/// - `expected_provider`: Provider identity bound to the invocation.
/// - `expected_model`: Model identity bound to the invocation.
/// - `expected_api`: API identifier bound to the invocation.
/// - `forwarded_settings`: Optional host observer for safe reports.
pub(super) fn capture_applied_settings(
    settings: AppliedRequestSettings,
    request_index: Option<usize>,
    captured_requests: &CapturedRequestSink,
    expected_provider: &str,
    expected_model: &str,
    expected_api: &str,
    forwarded_settings: Option<&AppliedRequestSettingsCallback>,
) {
    if settings.schema_version != "rpi-applied-request-settings-v1"
        || settings.provider_id != expected_provider
        || settings.model_id != expected_model
        || settings.api_id != expected_api
        || (!settings.complete && settings.omitted_categories.is_empty())
    {
        return;
    }
    if let (Some(index), Ok(mut requests)) = (request_index, captured_requests.lock())
        && let Some(request) = requests.get_mut(index)
        && let Ok(value) = serde_json::to_value(&settings)
    {
        request.applied_settings_fingerprint = applied_settings_fingerprint(&value);
        request.request_fingerprint = request_fingerprint_with_applied_settings(
            request.request_fingerprint.as_deref(),
            &value,
        );
        request.applied_settings = Some(value);
    }
    if let Some(callback) = forwarded_settings {
        callback(settings);
    }
}

/// Fingerprints one serialized upstream report without interpreting its fields.
///
/// # Parameters
/// - `settings`: Sanitized upstream report captured after provider preparation.
///
/// # Returns
/// A versioned BLAKE3 digest, or `None` if serialization fails.
fn applied_settings_fingerprint(settings: &serde_json::Value) -> Option<String> {
    let material = serde_json::to_vec(&("scriberig-applied-settings-v1", settings)).ok()?;
    Some(blake3::hash(&material).to_hex().to_string())
}

/// Adds the upstream applied-settings report to the request identity.
///
/// The report may be partial, so this fingerprint identifies captured
/// provenance and does not claim full provider-request equivalence.
///
/// # Parameters
/// - `request_fingerprint`: Existing `ScribeRig` input and model fingerprint.
/// - `settings`: Safe report captured at the provider preparation boundary.
///
/// # Returns
/// A versioned digest combining the base fingerprint and report, or `None`
/// when the base fingerprint is unavailable or serialization fails.
fn request_fingerprint_with_applied_settings(
    request_fingerprint: Option<&str>,
    settings: &serde_json::Value,
) -> Option<String> {
    let request_fingerprint = request_fingerprint?;
    let material = serde_json::to_vec(&(
        "scriberig-request-with-applied-settings-v1",
        request_fingerprint,
        settings,
    ))
    .ok()?;
    Some(blake3::hash(&material).to_hex().to_string())
}

//! Captures the upstream provider's safe applied-settings report.

use rpi_ai::Model;
use rpi_ai::{AppliedRequestSettings, AppliedRequestSettingsCallback};

use super::CapturedRequestSink;

/// Supplies exact provider identity and persistence sinks for one report.
#[derive(Clone, Copy)]
pub(super) struct AppliedSettingsCapture<'a> {
    /// Captured request updated by the report.
    pub(super) request_index: Option<usize>,
    /// Sink for bounded request provenance.
    pub(super) captured_requests: &'a CapturedRequestSink,
    /// Provider identity bound to the invocation.
    pub(super) expected_provider: &'a str,
    /// Model identity bound to the invocation.
    pub(super) expected_model: &'a str,
    /// API identifier bound to the invocation.
    pub(super) expected_api: &'a str,
    /// Descriptor expected for the resolved model.
    pub(super) expected_model_descriptor: &'a Model,
    /// Optional host observer for safe reports.
    pub(super) forwarded_settings: Option<&'a AppliedRequestSettingsCallback>,
}

/// Records an upstream applied-settings report after validating model identity.
///
/// # Parameters
/// - `settings`: Upstream report about transformed provider request settings.
/// - `request_index`: Captured request updated by the report.
/// - `captured_requests`: Sink for bounded request provenance.
/// - `expected_provider`: Provider identity bound to the invocation.
/// - `expected_model`: Model identity bound to the invocation.
/// - `expected_api`: API identifier bound to the invocation.
/// - `expected_model_descriptor`: Descriptor expected for the resolved model.
/// - `forwarded_settings`: Optional host observer for safe reports.
/// - `capture`: Request index, exact identity, and capture/observer sinks.
pub(super) fn capture_applied_settings(
    settings: AppliedRequestSettings,
    capture: AppliedSettingsCapture<'_>,
) {
    let AppliedSettingsCapture {
        request_index,
        captured_requests,
        expected_provider,
        expected_model,
        expected_api,
        expected_model_descriptor,
        forwarded_settings,
    } = capture;
    let supported_schema = matches!(
        settings.schema_version.as_str(),
        "rpi-applied-request-settings-v1"
            | "rpi-applied-request-settings-v2"
            | "rpi-applied-request-settings-v3"
            | "rpi-applied-request-settings-v4"
            | "rpi-applied-request-settings-v5"
    );
    let valid_body_digest = settings
        .request_body_sha256
        .as_deref()
        .is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    let valid_header_digest = settings
        .request_header_sha256
        .as_deref()
        .is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    let valid_effect_digest = settings
        .request_effect_sha256
        .as_deref()
        .is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    if !supported_schema
        || (settings.schema_version == "rpi-applied-request-settings-v1"
            && (settings.request_body_sha256.is_some() || settings.request_header_sha256.is_some()))
        || (settings.schema_version == "rpi-applied-request-settings-v2"
            && (!valid_body_digest || settings.request_header_sha256.is_some()))
        || (settings.schema_version == "rpi-applied-request-settings-v3"
            && (!valid_body_digest || !valid_header_digest))
        || (settings.schema_version == "rpi-applied-request-settings-v4"
            && (!valid_body_digest
                || !valid_header_digest
                || !rpi_ai::validate_provider_option_attestation(
                    expected_model_descriptor,
                    &settings.provider_options,
                )))
        || (settings.schema_version == "rpi-applied-request-settings-v5"
            && (!valid_body_digest
                || !valid_header_digest
                || (settings.request_effect_sha256.is_some() && !valid_effect_digest)
                || (settings.complete && !valid_effect_digest)
                || !rpi_ai::validate_provider_option_attestation(
                    expected_model_descriptor,
                    &settings.provider_options,
                )))
        || (settings.schema_version != "rpi-applied-request-settings-v5"
            && settings.request_effect_sha256.is_some())
        || settings.provider_id != expected_provider
        || settings.model_id != expected_model
        || settings.api_id != expected_api
        || (!settings.complete && settings.omitted_categories.is_empty())
        || (settings.complete
            && (settings.schema_version != "rpi-applied-request-settings-v5"
                || !settings.omitted_categories.is_empty()))
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

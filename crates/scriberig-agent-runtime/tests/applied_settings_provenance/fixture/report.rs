//! Builds versioned applied-settings reports for provenance test scenarios.

use std::collections::BTreeMap;

use rpi_ai::{AppliedRequestSettings, Model, ProviderOptionsAttestation};

use super::mode::ReportMode;

/// Builds one applied-settings report for a deterministic fixture mode.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
/// - `model`: Exact model identity bound to the report.
/// - `applied_values`: Safe scalar values advertised by the fixture.
///
/// # Returns
/// A report whose fields match the selected scenario.
pub(super) fn build(
    mode: ReportMode,
    model: &Model,
    applied_values: BTreeMap<String, serde_json::Value>,
) -> AppliedRequestSettings {
    AppliedRequestSettings {
        schema_version: schema_version(mode),
        provider_id: provider_id(mode, model),
        model_id: model_id(mode, model),
        api_id: api_id(mode, model),
        applied_values,
        request_body_sha256: body_digest(mode),
        request_header_sha256: header_digest(mode),
        request_effect_sha256: effect_digest(mode),
        provider_options: provider_options(mode, model),
        omitted_categories: omissions(mode),
        complete: complete(mode),
    }
}

/// Selects the report schema version for one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// The versioned report schema identifier.
fn schema_version(mode: ReportMode) -> String {
    match mode {
        ReportMode::UnknownSchema => "rpi-applied-request-settings-v6",
        ReportMode::AttestedV5
        | ReportMode::PartialV5
        | ReportMode::InvalidV5
        | ReportMode::CompleteWithOmissionsV5 => "rpi-applied-request-settings-v5",
        ReportMode::AttestedV4 | ReportMode::AttestedV4Transformed | ReportMode::InvalidV4 => {
            "rpi-applied-request-settings-v4"
        }
        ReportMode::AttestedV3 | ReportMode::InvalidV3 => "rpi-applied-request-settings-v3",
        ReportMode::AttestedV2 | ReportMode::InvalidV2 => "rpi-applied-request-settings-v2",
        _ => "rpi-applied-request-settings-v1",
    }
    .to_owned()
}

/// Selects the provider identity reported by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
/// - `model`: Exact model identity bound to the report.
///
/// # Returns
/// The expected or intentionally mismatched provider identity.
fn provider_id(mode: ReportMode, model: &Model) -> String {
    if matches!(mode, ReportMode::OtherProvider) {
        "other-provider".to_owned()
    } else {
        model.provider.clone()
    }
}

/// Selects the model identity reported by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
/// - `model`: Exact model identity bound to the report.
///
/// # Returns
/// The expected or intentionally mismatched model identity.
fn model_id(mode: ReportMode, model: &Model) -> String {
    if matches!(mode, ReportMode::OtherModel) {
        "other-model".to_owned()
    } else {
        model.id.clone()
    }
}

/// Selects the API identity reported by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
/// - `model`: Exact model identity bound to the report.
///
/// # Returns
/// The expected or intentionally mismatched API identity.
fn api_id(mode: ReportMode, model: &Model) -> String {
    if matches!(mode, ReportMode::OtherApi) {
        "other-api".to_owned()
    } else {
        model.api.as_str().to_owned()
    }
}

/// Selects the body digest represented by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// A valid, malformed, or absent body digest for the scenario.
fn body_digest(mode: ReportMode) -> Option<String> {
    match mode {
        ReportMode::AttestedV2
        | ReportMode::AttestedV3
        | ReportMode::InvalidV3
        | ReportMode::AttestedV4
        | ReportMode::AttestedV4Transformed
        | ReportMode::InvalidV4
        | ReportMode::AttestedV5
        | ReportMode::PartialV5
        | ReportMode::InvalidV5
        | ReportMode::CompleteWithOmissionsV5 => Some("a".repeat(64)),
        ReportMode::InvalidV2 => Some("invalid".to_owned()),
        _ => None,
    }
}

/// Selects the safe-header digest represented by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// A valid, malformed, or absent header digest for the scenario.
fn header_digest(mode: ReportMode) -> Option<String> {
    match mode {
        ReportMode::AttestedV3
        | ReportMode::AttestedV4
        | ReportMode::AttestedV4Transformed
        | ReportMode::InvalidV4
        | ReportMode::AttestedV5
        | ReportMode::PartialV5
        | ReportMode::InvalidV5
        | ReportMode::CompleteWithOmissionsV5 => Some("b".repeat(64)),
        ReportMode::InvalidV3 => Some("invalid".to_owned()),
        _ => None,
    }
}

/// Selects the final-request effect digest represented by one scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// A valid, malformed, or absent effect digest for the scenario.
fn effect_digest(mode: ReportMode) -> Option<String> {
    match mode {
        ReportMode::AttestedV5 | ReportMode::CompleteWithOmissionsV5 => Some("c".repeat(64)),
        ReportMode::InvalidV5 => Some("invalid".to_owned()),
        _ => None,
    }
}

/// Builds provider-option evidence for schema-aware report scenarios.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
/// - `model`: Exact model schema and options being attested.
///
/// # Returns
/// The upstream-generated option attestation for the scenario.
fn provider_options(mode: ReportMode, model: &Model) -> ProviderOptionsAttestation {
    if !matches!(
        mode,
        ReportMode::AttestedV4
            | ReportMode::AttestedV4Transformed
            | ReportMode::InvalidV4
            | ReportMode::AttestedV5
            | ReportMode::PartialV5
            | ReportMode::InvalidV5
            | ReportMode::CompleteWithOmissionsV5
    ) {
        return ProviderOptionsAttestation::default();
    }
    let effective = model
        .provider_options
        .iter()
        .map(|(key, value)| {
            let value = if matches!(mode, ReportMode::AttestedV4Transformed)
                && key == "future_vendor_mode"
            {
                serde_json::json!("accurate")
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect();
    let mut attestation = rpi_ai::attest_provider_options(model, &effective);
    if matches!(mode, ReportMode::InvalidV4) && !attestation.options.is_empty() {
        attestation.options[0].option_id_sha256 = "c".repeat(64);
    }
    attestation
}

/// Selects omission categories for one report scenario.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// The omitted categories declared by the scenario.
fn omissions(mode: ReportMode) -> Vec<String> {
    match mode {
        ReportMode::AttestedV5 | ReportMode::Unattested => Vec::new(),
        ReportMode::PartialV5 => {
            let mut categories = standard_omissions();
            categories.push("request_effect_identity".to_owned());
            categories
        }
        ReportMode::AttestedV4 | ReportMode::AttestedV4Transformed | ReportMode::InvalidV5 => {
            standard_omissions()
                .into_iter()
                .filter(|category| category != "opaque_provider_options")
                .collect()
        }
        _ => standard_omissions(),
    }
}

/// Returns the canonical omissions for a partial upstream request report.
///
/// # Returns
/// Standard categories omitted from request provenance.
fn standard_omissions() -> Vec<String> {
    [
        "endpoint",
        "headers_and_credentials",
        "prompt_and_transcript",
        "tools_and_schemas",
        "opaque_provider_options",
        "unrecognized_request_fields",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Indicates whether one scenario claims complete request coverage.
///
/// # Parameters
/// - `mode`: Report scenario being exercised.
///
/// # Returns
/// `true` when the scenario intentionally claims a complete report.
fn complete(mode: ReportMode) -> bool {
    matches!(
        mode,
        ReportMode::AttestedV5 | ReportMode::InvalidV5 | ReportMode::CompleteWithOmissionsV5
    )
}

//! Formats safe provider readiness metadata for upstream selector rows.

/// Summarizes credential readiness for all models under one provider.
///
/// # Parameters
/// - `library`: Model metadata and safe readiness rows.
/// - `provider`: Upstream provider identity.
///
/// # Returns
/// A short credential status without source details.
pub(super) fn provider_status(
    library: &scriberig_protocol::ModelLibraryResponse,
    provider: &str,
) -> &'static str {
    let statuses = library
        .provider_readiness
        .iter()
        .filter(|status| status.provider_id == provider)
        .map(|status| status.credential)
        .collect::<Vec<_>>();
    if statuses.is_empty() {
        return "credential status unavailable";
    }
    if statuses.contains(&scriberig_protocol::ProviderCredentialReadiness::Denied) {
        return "authorization denied";
    }
    if statuses.contains(&scriberig_protocol::ProviderCredentialReadiness::Available) {
        return "credential available";
    }
    if statuses.contains(&scriberig_protocol::ProviderCredentialReadiness::NotRequired) {
        return "authentication not required";
    }
    if statuses.contains(&scriberig_protocol::ProviderCredentialReadiness::Unsupported) {
        return "credential source unsupported";
    }
    if statuses.contains(&scriberig_protocol::ProviderCredentialReadiness::CatalogHeader) {
        return "catalog authentication configured";
    }
    "credential missing"
}

/// Looks up readiness for one exact provider/model endpoint.
///
/// # Parameters
/// - `library`: Model metadata and safe readiness rows.
/// - `provider`: Upstream provider identity.
/// - `model`: Provider-native model identity.
///
/// # Returns
/// A short credential status without source details.
pub(super) fn model_status(
    library: &scriberig_protocol::ModelLibraryResponse,
    provider: &str,
    model: &str,
) -> &'static str {
    let Some(status) = library
        .provider_readiness
        .iter()
        .find(|status| status.provider_id == provider && status.model_id == model)
    else {
        return "credential status unavailable";
    };
    match status.credential {
        scriberig_protocol::ProviderCredentialReadiness::NotRequired => {
            "authentication not required"
        }
        scriberig_protocol::ProviderCredentialReadiness::Available => "credential available",
        scriberig_protocol::ProviderCredentialReadiness::Missing => "credential missing",
        scriberig_protocol::ProviderCredentialReadiness::Denied => "authorization denied",
        scriberig_protocol::ProviderCredentialReadiness::Unsupported => {
            "credential source unsupported"
        }
        scriberig_protocol::ProviderCredentialReadiness::CatalogHeader => {
            "catalog authentication configured"
        }
    }
}

/// Sanitizes and bounds user-visible selector text.
///
/// # Parameters
/// - `value`: External identifier or display text.
///
/// # Returns
/// Safe text with at most 120 Unicode scalar values.
pub(super) fn safe_selector_text(value: &str) -> String {
    crate::inspector::sanitize(value)
        .chars()
        .take(120)
        .collect()
}

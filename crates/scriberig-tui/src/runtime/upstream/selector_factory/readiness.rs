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
    let authentication = library
        .provider_readiness
        .iter()
        .filter(|status| status.provider_id == provider)
        .filter_map(|status| status.authentication)
        .collect::<Vec<_>>();
    if authentication.contains(&scriberig_protocol::ProviderAuthenticationStatus::Rejected) {
        return "last request rejected by provider";
    }
    if authentication.contains(&scriberig_protocol::ProviderAuthenticationStatus::Accepted) {
        return "last request accepted by provider";
    }
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
    match status.authentication {
        Some(scriberig_protocol::ProviderAuthenticationStatus::Accepted) => {
            return "last request accepted by provider";
        }
        Some(scriberig_protocol::ProviderAuthenticationStatus::Rejected) => {
            return "last request rejected by provider";
        }
        None => {}
    }
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

#[cfg(test)]
mod tests {
    use super::{model_status, provider_status};
    use scriberig_protocol::{
        ModelLibraryResponse, ProviderAuthenticationStatus, ProviderCredentialReadiness,
        ProviderReadiness,
    };

    /// Builds a readiness projection with one upstream-authored outcome.
    ///
    /// # Parameters
    /// - `authentication`: Latest provider-authenticated request outcome.
    ///
    /// # Returns
    /// A one-row safe model-library response.
    fn library(authentication: ProviderAuthenticationStatus) -> ModelLibraryResponse {
        ModelLibraryResponse {
            models: Vec::new(),
            provider_readiness: vec![ProviderReadiness {
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                credential: ProviderCredentialReadiness::Available,
                authentication: Some(authentication),
            }],
        }
    }

    /// Keeps provider rejection distinct from local credential availability.
    #[test]
    fn model_and_provider_rows_show_upstream_authentication_rejection() {
        let library = library(ProviderAuthenticationStatus::Rejected);
        assert_eq!(
            model_status(&library, "provider-a", "model-a"),
            "last request rejected by provider"
        );
        assert_eq!(
            provider_status(&library, "provider-a"),
            "last request rejected by provider"
        );
    }

    /// Shows successful provider authentication separately from credential presence.
    #[test]
    fn model_status_shows_upstream_authentication_acceptance() {
        let library = library(ProviderAuthenticationStatus::Accepted);
        assert_eq!(
            model_status(&library, "provider-a", "model-a"),
            "last request accepted by provider"
        );
    }
}

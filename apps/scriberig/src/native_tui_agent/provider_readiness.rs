//! Projects upstream credential readiness through `ScribeRig` host authorization.

use std::collections::BTreeSet;

use rpi_model_runtime::CredentialSource as UpstreamCredentialSource;

use super::NativeTuiAgentRunner;

/// Builds credential-safe status rows for each exact provider model endpoint.
///
/// Upstream classifies catalog-supplied header auth. `ScribeRig` separately
/// resolves its authorized secret reference against each model endpoint. Only
/// the resulting readiness category leaves this module.
///
/// - `runner`: Provider catalog and host-authorized credential source.
///
/// Per-model readiness rows with no endpoint or credential values.
/// # Returns
/// The safe readiness projection.
pub(super) fn project(runner: &NativeTuiAgentRunner) -> Vec<scriberig_protocol::ProviderReadiness> {
    let models = runner.model_catalog.snapshot().models;
    let identities = models
        .iter()
        .map(|model| (model.provider.clone(), model.id.clone()))
        .collect::<BTreeSet<_>>();

    identities
        .into_iter()
        .map(|(provider_id, model_id)| {
            let model = models
                .iter()
                .find(|model| model.provider == provider_id && model.id == model_id)
                .expect("readiness identity came from this model snapshot");
            let source = match runner.upstream_credentials.as_deref() {
                Some(credentials) => {
                    scriberig_agent_runtime::protected_credential::credential_source_for_model(
                        credentials,
                        model,
                    )
                    .map_err(|_| ())
                }
                None => Ok(rpi_model_runtime::readiness_from_catalog(
                    &provider_id,
                    UpstreamCredentialSource::Missing,
                    std::slice::from_ref(model),
                )),
            };
            let credential = match source {
                Err(()) => scriberig_protocol::ProviderCredentialReadiness::Denied,
                Ok(
                    UpstreamCredentialSource::HostSelected
                    | UpstreamCredentialSource::CliOverride
                    | UpstreamCredentialSource::StoredApiKey
                    | UpstreamCredentialSource::StoredOauth
                    | UpstreamCredentialSource::ModelsJson
                    | UpstreamCredentialSource::Environment(_),
                ) => scriberig_protocol::ProviderCredentialReadiness::Available,
                Ok(UpstreamCredentialSource::Headers) => {
                    scriberig_protocol::ProviderCredentialReadiness::CatalogHeader
                }
                Ok(UpstreamCredentialSource::Anonymous) => {
                    scriberig_protocol::ProviderCredentialReadiness::NotRequired
                }
                Ok(UpstreamCredentialSource::Missing) => {
                    scriberig_protocol::ProviderCredentialReadiness::Missing
                }
            };
            scriberig_protocol::ProviderReadiness {
                provider_id,
                model_id,
                credential,
            }
        })
        .collect()
}

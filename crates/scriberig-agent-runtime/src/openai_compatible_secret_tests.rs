//! Verifies provider credentials stay outside captured safe model requests.

use std::sync::{Arc, Mutex};

use super::*;
use crate::cache::{CachePolicy, ModelCache};
use crate::model_provider::ProviderModel;
use crate::{AgentMessage, ModelDecision, ModelProvider};
use scriberig_secrets::{CredentialProvider, SecretError, SecretValue};

/// Returns one fake credential for an authorized provider destination.
struct FixtureCredentialProvider;

impl CredentialProvider for FixtureCredentialProvider {
    /// Resolves the fixture credential for a configured provider endpoint.
    ///
    /// - `endpoint`: Configured provider destination.
    ///
    /// The fake protected credential.
    ///
    /// Returns endpoint normalization or secret validation failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn credential(&self, endpoint: &str) -> Result<Option<SecretValue>, SecretError> {
        scriberig_secrets::normalize_endpoint(endpoint)?;
        SecretValue::new("FAKE_PROVIDER_SECRET_73B2".to_owned()).map(Some)
    }
}

/// Keeps provider credentials out of captured requests and redacts reflections.
#[test]
fn provider_secret_is_absent_from_safe_model_request_projection() {
    let secret = "FAKE_PROVIDER_SECRET_73B2";
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = MockCompletionTransport {
        status: reqwest::StatusCode::OK,
        body: serde_json::json!({
            "choices": [{
                "message": {"content": format!("{secret} answer")},
                "finish_reason": "stop"
            }]
        })
        .to_string()
        .into_bytes(),
        request_id: Some(secret.to_owned()),
        requests: requests.clone(),
    };
    let config = ProviderConfig::new(
        "fixture",
        "https://provider.example/v1",
        "model",
        CredentialSource::Authorized(Arc::new(FixtureCredentialProvider)),
    );
    let provider =
        OpenAiCompatibleProvider::with_transport(config, Arc::new(transport)).expect("provider");
    let mut model = ProviderModel::new(provider, "model")
        .with_cache(ModelCache::new(), CachePolicy::RecordOnly);

    let decision = model
        .decide(&[AgentMessage::User("inspect safely".to_owned())])
        .expect("model decision");
    let projections = model.take_request_projections();
    let captured = requests.lock().expect("captured requests");

    assert_eq!(
        decision,
        ModelDecision::Final("[REDACTED] answer".to_owned())
    );
    assert_eq!(projections.len(), 1);
    assert!(!format!("{:?}", projections[0]).contains(secret));
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].credential.as_deref(), Some(secret));
    assert!(!captured[0].payload.to_string().contains(secret));
}

//! Verifies safe provider configuration behavior.

use scriberig_agent_runtime::provider::{CredentialSource, ProviderConfig, ProviderError};

/// Verifies missing environment credentials become structured configuration errors.
#[test]
fn missing_environment_credential_is_configuration_error() {
    let config = ProviderConfig::new(
        "test",
        "http://127.0.0.1:1",
        "model",
        CredentialSource::Environment(
            "SCRIBERIG_TEST_PROVIDER_ENVIRONMENT_VARIABLE_THAT_IS_MISSING".to_owned(),
        ),
    );

    let error = config.resolve_credential().expect_err("missing credential");

    assert!(matches!(error, ProviderError::Configuration(_)));
}

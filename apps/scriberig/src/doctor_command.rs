//! Tests the effective provider API and credential through the runtime adapter.

use scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider;
use scriberig_agent_runtime::provider::{ProviderAdapter, ProviderConfig, ProviderError};

/// Runs a bounded live API request and renders a safe diagnostic report.
///
/// # Parameters
/// - `args`: Output format selected by the caller.
///
/// # Returns
/// Whether configuration, credential authorization, and the API check passed.
pub fn run(args: &scriberig_cli::DoctorArgs) -> bool {
    let result = crate::agent_config::resolve_provider_config();
    let (provider, credential_configured) = match result {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            print_report(
                args.format,
                &report(
                    "failed",
                    None,
                    None,
                    None,
                    "not configured",
                    "not run",
                    Some("no provider base_url is configured"),
                ),
            );
            return false;
        }
        Err(error) => {
            print_report(
                args.format,
                &report(
                    "failed",
                    None,
                    None,
                    None,
                    "not verified",
                    "not run",
                    Some(&error.message),
                ),
            );
            return false;
        }
    };
    run_provider_check(args.format, provider, credential_configured)
}

/// Checks one configured provider and reports the safe outcome.
///
/// # Parameters
/// - `format`: Human-readable or JSON output selection.
/// - `config`: Effective provider configuration without secret contents.
/// - `credential_configured`: Whether a secret reference was selected.
///
/// # Returns
/// Whether the configured API request succeeded.
fn run_provider_check(
    format: scriberig_cli::RunFormat,
    config: ProviderConfig,
    credential_configured: bool,
) -> bool {
    let provider_name = config.profile_id.clone();
    let model = config.model.clone();
    let endpoint = config.base_url.clone();
    let connection =
        OpenAiCompatibleProvider::new(config).and_then(|provider| test_provider(&provider, &model));
    let (status, credential, api, message) = match connection {
        Ok(()) => (
            "passed",
            if credential_configured {
                "accepted"
            } else {
                "not configured"
            },
            "passed",
            None,
        ),
        Err(error) => (
            "failed",
            credential_failure_status(credential_configured, &error),
            if matches!(
                &error,
                scriberig_agent_runtime::provider::ProviderError::Configuration(_)
            ) {
                "not reached"
            } else {
                "failed"
            },
            Some(error.to_string()),
        ),
    };
    print_report(
        format,
        &report(
            status,
            Some(&provider_name),
            Some(&model),
            Some(&endpoint),
            credential,
            api,
            message.as_deref(),
        ),
    );
    status == "passed"
}

/// Sends a one-token completion without displaying the generated text.
///
/// # Parameters
/// - `provider`: Configured adapter using the normal credential policy.
/// - `model`: Effective provider model identifier.
///
/// # Returns
/// Unit after the provider returns a valid normalized response.
///
/// # Errors
/// Returns a classified credential, transport, HTTP, or response error.
fn test_provider(
    provider: &dyn ProviderAdapter,
    model: &str,
) -> Result<(), scriberig_agent_runtime::provider::ProviderError> {
    provider.check_connection(model)
}

/// Classifies whether the selected reference was rejected or could not be used.
///
/// # Parameters
/// - `configured`: Whether configuration selected a credential reference.
/// - `error`: Safe provider failure category.
///
/// # Returns
/// A secret-free credential status label.
fn credential_failure_status(configured: bool, error: &ProviderError) -> &'static str {
    if configured {
        match error {
            ProviderError::Authentication(_) | ProviderError::Permission(_) => "rejected",
            ProviderError::Configuration(_) => "unavailable or unauthorized",
            _ => "not verified",
        }
    } else {
        "not configured"
    }
}

/// Creates a stable report object containing no credential material.
///
/// # Parameters
/// - `status`: Overall diagnostic outcome.
/// - `provider`: Safe provider profile identifier.
/// - `model`: Effective model identifier.
/// - `endpoint`: Validated provider base URL.
/// - `credential`: Credential status label.
/// - `api`: API connectivity status label.
/// - `message`: Optional safe diagnostic detail.
///
/// # Returns
/// A machine-readable secret-free diagnostic report.
fn report(
    status: &str,
    provider: Option<&str>,
    model: Option<&str>,
    endpoint: Option<&str>,
    credential: &str,
    api: &str,
    message: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "provider": provider,
        "model": model,
        "endpoint": endpoint,
        "credential": credential,
        "api": api,
        "message": message,
    })
}

/// Prints the diagnostic report in the requested representation.
///
/// # Parameters
/// - `format`: Human-readable or JSON output selection.
/// - `value`: Secret-free report object.
fn print_report(format: scriberig_cli::RunFormat, value: &serde_json::Value) {
    if matches!(format, scriberig_cli::RunFormat::Json) {
        println!("{value}");
        return;
    }
    println!("doctor: {}", value["status"].as_str().unwrap_or("failed"));
    for field in ["provider", "model", "endpoint", "credential", "api"] {
        if let Some(value) = value[field].as_str() {
            println!("{field}: {value}");
        }
    }
    if let Some(message) = value["message"].as_str() {
        println!("detail: {message}");
    }
}

#[cfg(test)]
mod tests {
    //! Verifies safe credential status classification in doctor reports.

    use super::credential_failure_status;
    use scriberig_agent_runtime::provider::ProviderError;

    /// Authentication failures identify a configured credential as rejected.
    #[test]
    fn rejected_credentials_are_classified_without_details() {
        let error = ProviderError::Authentication("private detail".to_owned());
        assert_eq!(credential_failure_status(true, &error), "rejected");
    }

    /// Missing credentials remain distinct from failed authentication.
    #[test]
    fn unconfigured_credentials_are_reported_as_unconfigured() {
        let error = ProviderError::Transport("offline".to_owned());
        assert_eq!(credential_failure_status(false, &error), "not configured");
    }
}

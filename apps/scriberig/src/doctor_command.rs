//! Tests the effective provider API and credential through the runtime adapter.

use scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider;
use scriberig_agent_runtime::provider::{ProviderAdapter, ProviderConfig, ProviderError};

/// Runs a bounded live API request and renders a safe diagnostic report.
///
/// - `args`: Output format selected by the caller.
///
/// Whether configuration, credential authorization, and the API check passed.
/// # Returns
/// The operation result.
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
                    Some((
                        "no provider base_url is configured",
                        "Configure a provider base URL and model, then retry.",
                    )),
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
                    Some((
                        &error.message,
                        "Run `srig config show` to inspect the effective settings.",
                    )),
                ),
            );
            return false;
        }
    };
    run_provider_check(args.format, provider, credential_configured)
}

/// Checks one configured provider and reports the safe outcome.
///
/// - `format`: Human-readable or JSON output selection.
/// - `config`: Effective provider configuration without secret contents.
/// - `credential_configured`: Whether a secret reference was selected.
///
/// Whether the configured API request succeeded.
/// # Returns
/// The operation result.
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
    let (status, credential, api, message, next_step) = match connection {
        Ok(()) => (
            "passed",
            if credential_configured {
                "accepted"
            } else {
                "not configured"
            },
            "passed",
            None,
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
            Some(failure_guidance(&error)),
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
            message.as_deref().zip(next_step),
        ),
    );
    status == "passed"
}

/// Sends a one-token completion without displaying the generated text.
///
/// - `provider`: Configured adapter using the normal credential policy.
/// - `model`: Effective provider model identifier.
///
/// Unit after the provider returns a valid normalized response.
///
/// Returns a classified credential, transport, HTTP, or response error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn test_provider(
    provider: &dyn ProviderAdapter,
    model: &str,
) -> Result<(), scriberig_agent_runtime::provider::ProviderError> {
    provider.check_connection(model)
}

/// Classifies whether the selected reference was rejected or could not be used.
///
/// - `configured`: Whether configuration selected a credential reference.
/// - `error`: Safe provider failure category.
///
/// A secret-free credential status label.
/// # Returns
/// The operation result.
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

/// Suggests a portable next step for one classified doctor failure.
///
/// - `error`: Provider failure category and safe diagnostic.
///
/// Actionable guidance that does not assume a particular host or provider.
/// # Returns
/// The operation result.
fn failure_guidance(error: &ProviderError) -> &'static str {
    match error {
        ProviderError::Configuration(_) => {
            "Review the effective provider, model, and endpoint settings."
        }
        ProviderError::Authentication(_) | ProviderError::Permission(_) => {
            "Verify the configured credential reference and provider access."
        }
        ProviderError::RateLimited(_) => {
            "Wait for the provider rate limit window to pass, then retry."
        }
        ProviderError::Timeout(_) => {
            "Check provider availability and load, then retry the bounded request."
        }
        ProviderError::Transport(message) if message.contains("connection refused") => {
            "Confirm a service is listening at the configured endpoint and that network policy permits access."
        }
        ProviderError::Transport(_) => {
            "Check endpoint reachability, network access, and TLS configuration."
        }
        ProviderError::InvalidRequest(_) => {
            "Check that the selected model supports the configured API request."
        }
        ProviderError::ServerFailure(_) => {
            "Check the provider service status and its server-side diagnostics."
        }
        ProviderError::MalformedResponse(_) => {
            "Check API compatibility and confirm the model can return the required marker."
        }
        ProviderError::Cancelled => "Retry the check after the cancellation condition is cleared.",
    }
}

/// Creates a stable report object containing no credential material.
///
/// - `status`: Overall diagnostic outcome.
/// - `provider`: Safe provider profile identifier.
/// - `model`: Effective model identifier.
/// - `endpoint`: Validated provider base URL.
/// - `credential`: Credential status label.
/// - `api`: API connectivity status label.
/// - `advice`: Optional safe diagnostic detail and portable next step.
///
/// A machine-readable secret-free diagnostic report.
/// # Returns
/// The operation result.
fn report(
    status: &str,
    provider: Option<&str>,
    model: Option<&str>,
    endpoint: Option<&str>,
    credential: &str,
    api: &str,
    advice: Option<(&str, &str)>,
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "provider": provider,
        "model": model,
        "endpoint": endpoint,
        "credential": credential,
        "api": api,
        "message": advice.map(|(message, _)| message),
        "next_step": advice.map(|(_, next_step)| next_step),
    })
}

/// Prints the diagnostic report in the requested representation.
///
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
    if let Some(next_step) = value["next_step"].as_str() {
        println!("next: {next_step}");
    }
}

#[cfg(test)]
mod tests {
    //! Verifies safe credential status classification in doctor reports.

    use super::{credential_failure_status, failure_guidance};
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

    /// Refused connections include actionable guidance without host assumptions.
    #[test]
    fn transport_failure_guidance_explains_refused_connections() {
        let error = ProviderError::Transport("connection refused".to_owned());

        assert_eq!(
            failure_guidance(&error),
            "Confirm a service is listening at the configured endpoint and that network policy permits access."
        );
    }
}

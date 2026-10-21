//! Tests the effective upstream provider API through host credential policy.

use rpi_ai::{Context, Message, Model, SimpleStreamOptions, StopReason, UserMessage};
use scriberig_agent_runtime::provider::HostConfigurationError;

#[derive(Debug)]
/// Describes a safe failure while checking the configured upstream provider.
///
/// # Public members
/// None.
///
/// # Private members
/// - `Configuration`: Safe host configuration failure.
/// - `Upstream`: Provider request failure without response details.
enum DoctorFailure {
    Configuration(HostConfigurationError),
    Upstream(String),
}

impl std::fmt::Display for DoctorFailure {
    /// Formats a safe diagnostic for the command line.
    ///
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// The formatter result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(error) => error.fmt(formatter),
            Self::Upstream(message) => {
                write!(formatter, "upstream provider check failed: {message}")
            }
        }
    }
}

/// Runs a bounded live API request and renders a safe diagnostic report.
///
/// - `args`: Output format selected by the caller.
///
/// Whether configuration, credential authorization, and the API check passed.
/// # Returns
/// The operation result.
pub fn run(args: &scriberig_cli::DoctorArgs) -> bool {
    let result = crate::agent_config::resolve_provider_config();
    let (model, credentials, credential_configured) = match result {
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
    run_provider_check(
        args.format,
        &model,
        credentials.as_deref(),
        credential_configured,
    )
}

/// Checks one configured provider and reports the safe outcome.
///
/// - `format`: Human-readable or JSON output selection.
/// - `model`: Exact upstream provider model selected by configuration.
/// - `credentials`: Host-authorized credential adapter, when configured.
/// - `credential_configured`: Whether a secret reference was selected.
///
/// Whether the configured API request succeeded.
/// # Returns
/// The operation result.
fn run_provider_check(
    format: scriberig_cli::RunFormat,
    model: &Model,
    credentials: Option<&dyn scriberig_secrets::CredentialProvider>,
    credential_configured: bool,
) -> bool {
    let provider_name = model.provider.clone();
    let model_id = model.id.clone();
    let endpoint = model.base_url.clone();
    let connection = test_provider(model, credentials);
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
            if matches!(&error, DoctorFailure::Configuration(_)) {
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
            Some(&model_id),
            Some(&endpoint),
            credential,
            api,
            message.as_deref().zip(next_step),
        ),
    );
    status == "passed"
}

/// Sends a one-token upstream completion without displaying generated text.
///
/// - `model`: Exact upstream provider model.
/// - `credentials`: Host-authorized credential adapter, when configured.
///
/// Unit after the provider returns a valid normalized response.
///
/// Returns a classified credential, transport, HTTP, or response error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn test_provider(
    model: &Model,
    credentials: Option<&dyn scriberig_secrets::CredentialProvider>,
) -> Result<(), DoctorFailure> {
    let provider = rpi_model_runtime::create_builtin_provider(
        &model.provider,
        vec![model.clone()],
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| {
                DoctorFailure::Configuration(HostConfigurationError::Configuration(
                    "provider client unavailable".to_owned(),
                ))
            })?,
    )
    .map_err(|error| {
        DoctorFailure::Configuration(HostConfigurationError::Configuration(error.to_string()))
    })?;
    let request_secret = match credentials {
        None => None,
        Some(credentials) => {
            scriberig_agent_runtime::protected_credential::request_secret_for_model(
                credentials,
                model,
            )
            .map_err(|error| {
                DoctorFailure::Configuration(HostConfigurationError::Configuration(
                    error.to_string(),
                ))
            })?
        }
    };
    let prompt = Context::new(vec![Message::User(UserMessage::new("Reply with OK.", 0))]);
    let options = SimpleStreamOptions {
        max_tokens: Some(1),
        timeout: Some(std::time::Duration::from_secs(60)),
        ..Default::default()
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| {
            DoctorFailure::Configuration(HostConfigurationError::Configuration(
                "async runtime unavailable".to_owned(),
            ))
        })?;
    let response = runtime.block_on(async {
        let events = match request_secret.as_ref() {
            Some(secret) => {
                provider
                    .stream_simple_with_request_secret(model, &prompt, &options, secret, true)
                    .await
            }
            None => provider.stream_simple(model, &prompt, &options).await,
        };
        events.result().await.map_err(|error| {
            DoctorFailure::Upstream(format!("provider response stream ended: {error}"))
        })
    })?;
    match response.stop_reason {
        StopReason::Error | StopReason::Aborted => Err(DoctorFailure::Upstream(
            "provider returned an error terminal message".to_owned(),
        )),
        _ => Ok(()),
    }
}

/// Classifies whether the selected reference was rejected or could not be used.
///
/// - `configured`: Whether configuration selected a credential reference.
/// - `error`: Safe provider failure category.
///
/// A secret-free credential status label.
/// # Returns
/// The operation result.
fn credential_failure_status(configured: bool, error: &DoctorFailure) -> &'static str {
    if configured {
        match error {
            DoctorFailure::Configuration(_) => "unavailable or unauthorized",
            DoctorFailure::Upstream(_) => "not verified",
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
fn failure_guidance(error: &DoctorFailure) -> &'static str {
    match error {
        DoctorFailure::Configuration(_) => {
            "Review the effective provider, model, and endpoint settings."
        }
        DoctorFailure::Upstream(_) => {
            "Check the provider endpoint and credential configuration, then retry the bounded request."
        }
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

    use super::DoctorFailure;
    use super::{credential_failure_status, failure_guidance};

    /// Authentication failures identify a configured credential as rejected.
    #[test]
    fn rejected_credentials_are_classified_without_details() {
        let error = DoctorFailure::Upstream("private detail".to_owned());
        assert_eq!(credential_failure_status(true, &error), "not verified");
    }

    /// Missing credentials remain distinct from failed authentication.
    #[test]
    fn unconfigured_credentials_are_reported_as_unconfigured() {
        let error = DoctorFailure::Upstream("offline".to_owned());
        assert_eq!(credential_failure_status(false, &error), "not configured");
    }

    /// Refused connections include actionable guidance without host assumptions.
    #[test]
    fn upstream_failure_guidance_does_not_invent_transport_classification() {
        let error = DoctorFailure::Upstream("connection refused".to_owned());

        assert_eq!(
            failure_guidance(&error),
            "Check the provider endpoint and credential configuration, then retry the bounded request."
        );
    }
}

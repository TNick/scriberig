//! Resolves the native agent provider from trusted process configuration.

use scriberig_agent_runtime::provider::HostConfigurationError;

use crate::native_scenario::NativeScenario;
use crate::self_hosting_provider::{CancellationPhase, ValidationMode};
#[path = "provider_config/catalog.rs"]
pub(super) mod catalog;
#[path = "provider_config/extensions.rs"]
pub(super) mod extensions;

/// Builds a deterministic scenario adapter when explicitly selected.
///
/// - `provider_kind`: Trusted provider selection from process configuration.
///
/// Configured deterministic provider, or `None` for unsupported modes.
///
/// Returns a structured error for an unsupported provider or scenario.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn from_environment(
    provider_kind: &str,
) -> Result<Option<NativeScenario>, HostConfigurationError> {
    match provider_kind {
        "deterministic" => deterministic_provider(),
        unknown => Err(HostConfigurationError::Configuration(format!(
            "unsupported SCRIBE_AGENT_PROVIDER: {unknown}"
        ))),
    }
}

/// Selects a registered deterministic provider scenario.
///
/// Configured deterministic scenario.
///
/// Returns a configuration error when the scenario identifier is unknown.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn deterministic_provider() -> Result<Option<NativeScenario>, HostConfigurationError> {
    let scenario =
        std::env::var("SCRIBE_AGENT_SCENARIO").unwrap_or_else(|_| "bootstrap".to_owned());
    let provider = match scenario.as_str() {
        "arithmetic" => {
            NativeScenario::Arithmetic(crate::deterministic_provider::ArithmeticProvider)
        }
        "bootstrap" => NativeScenario::Bootstrap(crate::deterministic_provider::BootstrapProvider),
        "bootstrap_import_failure" => NativeScenario::BootstrapImportFailure(
            crate::bootstrap_failure_provider::BootstrapImportFailureProvider,
        ),
        "self_hosting_php" => self_hosting(ValidationMode::Normal, None),
        "self_hosting_php_validation_failure" => self_hosting(ValidationMode::FailFirst, None),
        "self_hosting_php_validation_repair" => self_hosting(ValidationMode::RepairFirst, None),
        "self_hosting_php_cancellation" => {
            self_hosting(ValidationMode::Normal, Some(CancellationPhase::Onboarding))
        }
        "self_hosting_php_discovery_cancellation" => {
            self_hosting(ValidationMode::Normal, Some(CancellationPhase::Discovery))
        }
        "self_hosting_php_implementation_cancellation" => self_hosting(
            ValidationMode::Normal,
            Some(CancellationPhase::Implementation),
        ),
        unknown => {
            return Err(HostConfigurationError::Configuration(format!(
                "unsupported SCRIBE_AGENT_SCENARIO: {unknown}"
            )));
        }
    };
    Ok(Some(provider))
}

/// Builds a PHP self-hosting scenario with explicit validation and cancellation behavior.
///
/// - `validation`: First-candidate validation behavior.
/// - `cancellation`: Optional delayed operation phase.
///
/// Provider-neutral deterministic PHP scenario.
/// # Returns
/// The operation result.
fn self_hosting(
    validation: ValidationMode,
    cancellation: Option<CancellationPhase>,
) -> NativeScenario {
    NativeScenario::SelfHostingPhp(crate::self_hosting_provider::PhpSelfHostingProvider {
        validation,
        cancellation,
    })
}

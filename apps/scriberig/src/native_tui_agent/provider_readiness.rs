//! Projects upstream credential readiness through `ScribeRig` host authorization.
//!
//! The shared runtime projection owns the credential-source classification and
//! binds each row to the upstream catalog generation it resolved. This module
//! only supplies the runner's shared catalog handle and its host-authorized
//! credential authority.

use super::NativeTuiAgentRunner;

/// Builds credential-safe status rows for each exact provider model endpoint.
///
/// - `runner`: Provider catalog and host-authorized credential source.
///
/// Per-model readiness rows with their catalog generation, and no endpoint or
/// credential values.
/// # Returns
/// The safe readiness projection.
pub(super) fn project(
    runner: &NativeTuiAgentRunner,
) -> scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
    scriberig_agent_runtime::provider_readiness::project_readiness(
        &runner.model_catalog,
        runner.upstream_credentials.as_deref(),
    )
}

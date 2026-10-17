//! Authorizes exact credential destinations and canonical project roots.

use crate::{SecretError, SecretRef};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::{Host, Url};

/// Distinguishes local interactive execution from service execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// A local user initiated execution.
    Interactive,
    /// A service handles remote execution.
    Service,
}

/// Describes the authority requested by one credential consumer.
#[derive(Clone, Debug)]
pub struct SecretUseContext {
    /// Canonical project root, matched exactly.
    pub project_root: PathBuf,
    /// Stable consumer purpose, currently `model_provider`.
    pub purpose: String,
    /// Execution mode subject to the binding.
    pub mode: ExecutionMode,
}

/// Grants use of one reference for one destination and project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Indirect secret reference.
    pub credential: SecretRef,
    /// Normalized exact provider base URL.
    pub base_url: String,
    /// Canonical project root.
    pub project_root: PathBuf,
    /// Permitted purpose.
    pub purpose: String,
    /// Permitted execution mode.
    pub mode: ExecutionMode,
}

/// Holds an explicit allowlist; an empty list grants no use.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretPolicy {
    /// Exact credential-use grants.
    pub bindings: Vec<Binding>,
}

impl SecretPolicy {
    /// Checks destination authority without accessing a secret source.
    ///
    /// # Parameters
    /// - `reference`: Requested secret source.
    /// - `endpoint`: Exact provider base URL.
    /// - `context`: Trusted project and consumer context.
    ///
    /// # Returns
    /// Unit when an exact binding grants the use.
    ///
    /// # Errors
    /// Returns `Denied` or `Invalid` before any secret or network access.
    pub fn authorize(
        &self,
        reference: &SecretRef,
        endpoint: &str,
        context: &SecretUseContext,
    ) -> Result<(), SecretError> {
        reference.validate()?;
        let endpoint = normalize_endpoint(endpoint)?;
        let root = context
            .project_root
            .canonicalize()
            .map_err(|_| SecretError::Denied)?;
        if self.bindings.iter().any(|binding| {
            binding.credential == *reference
                && binding.base_url == endpoint
                && binding.project_root == root
                && binding.purpose == context.purpose
                && binding.mode == context.mode
        }) {
            Ok(())
        } else {
            Err(SecretError::Denied)
        }
    }
}

/// Normalizes an exact provider URL and rejects insecure credential targets.
///
/// # Parameters
/// - `endpoint`: Provider base URL before adding transport paths.
///
/// # Returns
/// A normalized URL without a trailing slash.
///
/// # Errors
/// Returns `Invalid` for credentials, queries, fragments, or insecure transport.
pub fn normalize_endpoint(endpoint: &str) -> Result<String, SecretError> {
    validate_public_endpoint(endpoint)?;
    let url = Url::parse(endpoint).map_err(|_| SecretError::Invalid)?;
    let numeric_loopback = match url.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    };
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (url.scheme() == "http" && numeric_loopback))
    {
        return Err(SecretError::Invalid);
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// Validates an endpoint descriptor before it can appear in configuration output.
///
/// # Parameters
/// - `endpoint`: Nonsecret provider endpoint configuration.
///
/// # Returns
/// Unit for HTTP(S) URLs without embedded credentials, queries, or fragments.
///
/// # Errors
/// Returns a safe `Invalid` category without retaining the raw URL.
pub fn validate_public_endpoint(endpoint: &str) -> Result<(), SecretError> {
    let url = Url::parse(endpoint).map_err(|_| SecretError::Invalid)?;
    if url.host().is_none()
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(SecretError::Invalid);
    }
    Ok(())
}

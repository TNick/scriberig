//! Checks live user and system policy before each credential read.

use super::RegistryLocations;
use crate::{
    Binding, CredentialProvider, SecretError, SecretPolicy, SecretRef, SecretResolver,
    SecretUseContext, SecretValue,
};
use std::sync::Arc;

/// Resolves a reference for one consumer after live policy authorization.
pub struct AuthorizedCredential {
    /// Requested reference, with no embedded value.
    pub reference: SecretRef,
    /// Trusted consumer identity and canonical project scope.
    pub context: SecretUseContext,
    /// Fixed policy locations selected by the composition root.
    pub locations: RegistryLocations,
    /// Optional grant derived solely from explicit local CLI intent.
    pub invocation_binding: Option<Binding>,
    /// Underlying reader; never called before authorization.
    pub resolver: Arc<dyn SecretResolver>,
}

impl CredentialProvider for AuthorizedCredential {
    /// Rechecks destination and project authority before reading the source.
    ///
    /// # Parameters
    /// - `endpoint`: Actual configured provider base URL.
    ///
    /// # Returns
    /// A protected credential for this request only.
    ///
    /// # Errors
    /// Returns a safe failure before retrieval if authority is absent.
    fn credential(&self, endpoint: &str) -> Result<Option<SecretValue>, SecretError> {
        let mut policy = self.locations.load()?.policy;
        if let Some(binding) = &self.invocation_binding {
            policy.bindings.push(binding.clone());
        }
        policy.authorize(&self.reference, endpoint, &self.context)?;
        if let Some(system) = self.locations.system()? {
            system.authorize(&self.reference, endpoint, &self.context)?;
        }
        self.resolver.resolve(&self.reference).map(Some)
    }
}

/// Checks an in-memory policy before a test or embedded source read.
///
/// # Parameters
/// - `policy`: Explicit allowed destinations and projects.
/// - `reference`: Requested reference.
/// - `endpoint`: Requested destination.
/// - `context`: Trusted consumer context.
/// - `resolver`: Secret source called only after authorization.
///
/// # Returns
/// A protected authorized credential.
///
/// # Errors
/// Returns a safe policy or source error.
pub fn resolve_authorized(
    policy: &SecretPolicy,
    reference: &SecretRef,
    endpoint: &str,
    context: &SecretUseContext,
    resolver: &dyn SecretResolver,
) -> Result<SecretValue, SecretError> {
    policy.authorize(reference, endpoint, context)?;
    resolver.resolve(reference)
}

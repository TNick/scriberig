//! Bridges destination-authorized `ScribeRig` credentials to upstream requests.

use std::sync::Arc;

use rpi_ai::{Model, RequestSecret, SecretMaterial};
use rpi_model_runtime::{
    CredentialRequestSource, CredentialResolutionError, CredentialSource, HostCredentialMaterial,
    ProtectedCredentialResolver, resolve_host_selected_credential_with_policy,
};
use scriberig_secrets::{CredentialProvider, SecretError, SecretValue};

/// Resolves a protected credential for one already-resolved upstream model.
///
/// Upstream requests the host-selected source through its protected resolver.
/// The request-scoped host resolver remains bound to the exact model endpoint,
/// and authorization or source errors stop resolution rather than becoming a
/// credential miss.
///
/// # Parameters
/// - `credentials`: `ScribeRig` credential authority for this invocation.
/// - `model`: Exact upstream model selected for the provider request.
///
/// # Returns
/// An opaque upstream request secret, or `None` when the authority explicitly
/// reports that no authentication is configured.
///
/// # Errors
/// Returns the safe classified credential error when authorization or lookup
/// fails.
pub fn request_secret_for_model(
    credentials: &dyn CredentialProvider,
    model: &Model,
) -> Result<Option<RequestSecret>, SecretError> {
    let resolver = authorized_resolver(credentials, model);
    let protected = resolve_host_selected_credential_with_policy(false, &resolver)
        .map_err(map_upstream_resolution_error)?;
    let Some(secret) = protected.material else {
        return Ok(None);
    };
    let material = SecretValueMaterial(secret);
    Ok(Some(RequestSecret::new(Arc::new(material))))
}

/// Resolves a safe upstream credential-source classification for readiness.
///
/// This uses the same request-scoped host resolver as provider execution and
/// drops the protected material before returning the classification.
///
/// # Parameters
/// - `credentials`: `ScribeRig` credential authority for this invocation.
/// - `model`: Exact upstream model selected for the provider request.
///
/// # Returns
/// Upstream's host-selected, catalog-header, or missing credential source.
///
/// # Errors
/// Returns the safe classified credential error when authorization or lookup
/// fails.
pub fn credential_source_for_model(
    credentials: &dyn CredentialProvider,
    model: &Model,
) -> Result<CredentialSource, SecretError> {
    let resolver = authorized_resolver(credentials, model);
    let protected = resolve_host_selected_credential_with_policy(false, &resolver)
        .map_err(map_upstream_resolution_error)?;
    Ok(rpi_model_runtime::readiness_from_catalog(
        &model.provider,
        protected.source,
        std::slice::from_ref(model),
    ))
}

/// Binds `ScribeRig` authorization to the exact selected upstream endpoint.
///
/// # Parameters
/// - `credentials`: Authorized `ScribeRig` credential source.
/// - `model`: Exact upstream model used to identify the destination endpoint.
///
/// # Returns
/// A resolver bound to the selected model endpoint.
fn authorized_resolver<'a>(
    credentials: &'a dyn CredentialProvider,
    model: &Model,
) -> AuthorizedSecretResolver<'a> {
    AuthorizedSecretResolver {
        credentials,
        endpoint: model.base_url.clone(),
    }
}

/// Resolves only the host-selected source for one authorized destination.
///
/// # Public members
/// None.
///
/// # Private members
/// - `credentials`: `ScribeRig`'s destination-authorizing credential provider.
/// - `endpoint`: Exact upstream model endpoint for this request.
struct AuthorizedSecretResolver<'a> {
    /// `ScribeRig` authority that rechecks destination policy before reading.
    credentials: &'a dyn CredentialProvider,
    /// Exact endpoint copied from the selected upstream model.
    endpoint: String,
}

impl ProtectedCredentialResolver<SecretValue> for AuthorizedSecretResolver<'_> {
    /// Resolves a legacy source request as a safe miss.
    ///
    /// The selected host credential is available only through the error-bearing
    /// `resolve_with_policy` method.
    ///
    /// # Parameters
    /// - `_source`: Non-secret source descriptor requested by upstream.
    ///
    /// # Returns
    /// No legacy source material.
    fn resolve(
        &self,
        _source: CredentialRequestSource,
    ) -> Option<HostCredentialMaterial<SecretValue>> {
        None
    }

    /// Resolves the single host-selected reference with fail-closed errors.
    ///
    /// # Parameters
    /// - `source`: Non-secret source descriptor requested by upstream.
    ///
    /// # Returns
    /// The authorized secret material, or a miss when no credential is set.
    ///
    /// # Errors
    /// Returns a safe denial or resolver failure without source fallthrough.
    fn resolve_with_policy(
        &self,
        source: CredentialRequestSource,
    ) -> Result<Option<HostCredentialMaterial<SecretValue>>, CredentialResolutionError> {
        if source != CredentialRequestSource::HostSelected {
            return Ok(None);
        }
        let secret = self
            .credentials
            .credential(&self.endpoint)
            .map_err(map_resolution_error)?;
        Ok(secret.map(|material| HostCredentialMaterial {
            material,
            source: CredentialSource::HostSelected,
        }))
    }
}

/// Maps host errors into upstream's non-secret fail-closed classifications.
///
/// # Parameters
/// - `error`: Safe `ScribeRig` credential error.
///
/// # Returns
/// An upstream denial or generic resolver failure.
fn map_resolution_error(error: SecretError) -> CredentialResolutionError {
    match error {
        SecretError::Denied | SecretError::Invalid => CredentialResolutionError::AccessDenied,
        SecretError::Missing
        | SecretError::Locked
        | SecretError::Unavailable
        | SecretError::Unsupported
        | SecretError::Conflict
        | SecretError::Partial => CredentialResolutionError::ResolverFailure,
    }
}

/// Restores a safe `ScribeRig` error from upstream's non-secret failure class.
///
/// # Parameters
/// - `error`: Upstream credential resolution failure.
///
/// # Returns
/// A safe host denial or availability failure.
fn map_upstream_resolution_error(error: CredentialResolutionError) -> SecretError {
    match error {
        CredentialResolutionError::AccessDenied => SecretError::Denied,
        _ => SecretError::Unavailable,
    }
}

/// Keeps a `ScribeRig` zeroizing secret behind upstream's synchronous secret API.
///
/// # Public members
/// None.
///
/// # Private members
/// - `0`: Zeroizing `ScribeRig` credential buffer.
struct SecretValueMaterial(SecretValue);

impl SecretMaterial for SecretValueMaterial {
    /// Borrows the secret only for the duration of the request consumer.
    ///
    /// # Parameters
    /// - `consumer`: Synchronous upstream request operation receiving the
    ///   borrowed credential.
    fn with_secret(&self, consumer: &mut dyn FnMut(&str)) {
        consumer(self.0.expose());
    }
}

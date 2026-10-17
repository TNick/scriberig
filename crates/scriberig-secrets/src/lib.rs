//! Provides protected credentials and explicit destination authorization.

mod backend;
mod error;
pub mod files;
mod policy;
pub mod process;
mod reference;
pub mod registry;
pub mod sensitive;
mod value;

pub use backend::{OsSecretStore, SecretResolver, SecretStore, SourceResolver};
pub use error::SecretError;
pub use policy::{
    Binding, ExecutionMode, SecretPolicy, SecretUseContext, normalize_endpoint,
    validate_public_endpoint,
};
pub use reference::SecretRef;
pub use value::SecretValue;

/// Resolves a credential only for the destination selected by its consumer.
pub trait CredentialProvider: Send + Sync {
    /// Authorizes and resolves one outbound credential.
    ///
    /// # Parameters
    /// - `endpoint`: Exact configured provider base URL.
    ///
    /// # Returns
    /// A protected credential, or no authentication when explicitly disabled.
    ///
    /// # Errors
    /// Returns a safe authorization or source failure.
    fn credential(&self, endpoint: &str) -> Result<Option<SecretValue>, SecretError>;
}

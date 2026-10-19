//! Defines secret-free model request identity fingerprints.

use crate::provider::ProviderCapabilities;

/// Inputs that contribute to exact model request identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestFingerprintInput {
    /// Fingerprint schema version.
    pub schema_version: String,
    /// Provider profile or compatibility identity.
    pub profile: String,
    /// Provider model identity.
    pub model: String,
    /// Provider capability descriptor.
    pub capabilities: ProviderCapabilities,
    /// Canonical compiled context.
    pub compiled_context: String,
    /// Canonical tool catalog identity.
    pub tools: String,
    /// Secret deliberately excluded from identity material.
    credential: Option<String>,
}

impl RequestFingerprintInput {
    /// Creates a fingerprint input with default capability metadata.
    ///
    /// - `profile`: Provider profile identity.
    /// - `model`: Provider model identity.
    /// - `compiled_context`: Canonical context.
    /// - `tools`: Canonical tool catalog identity.
    ///
    /// Fingerprint inputs with secret-free identity material.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(
        profile: impl Into<String>,
        model: impl Into<String>,
        compiled_context: impl Into<String>,
        tools: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: "model-request-v1".to_owned(),
            profile: profile.into(),
            model: model.into(),
            capabilities: ProviderCapabilities::default(),
            compiled_context: compiled_context.into(),
            tools: tools.into(),
            credential: None,
        }
    }

    /// Replaces the model identity for a fingerprint comparison.
    ///
    /// - `self`: Fingerprint input.
    /// - `model`: New model identity.
    ///
    /// Inputs using the new model identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Adds a secret only to prove it is excluded from identity material.
    ///
    /// - `self`: Fingerprint input.
    /// - `credential`: Secret value never hashed or serialized.
    ///
    /// Inputs retaining the secret outside canonical material.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_credential(mut self, credential: impl Into<String>) -> Self {
        self.credential = Some(credential.into());
        self
    }

    /// Computes the canonical content fingerprint.
    ///
    /// Lowercase BLAKE3 digest over secret-free request identity material.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn fingerprint(&self) -> String {
        let material = format!(
            "{}\n{}\n{}\n{:?}\n{}\n{}",
            self.schema_version,
            self.profile,
            self.model,
            self.capabilities,
            self.compiled_context,
            self.tools
        );
        blake3::hash(material.as_bytes()).to_hex().to_string()
    }
}

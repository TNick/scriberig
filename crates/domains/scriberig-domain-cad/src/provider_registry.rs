//! Composes optional external CAD providers at the shared application boundary.

use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::{
    CadExternalProvider, CadFormat, CadProviderDescriptor, CadProviderErrorKind, CadProviderStatus,
    CadProviderUnavailable,
};

/// Configures provider instances and initialization failures by external CAD format.
///
/// Public members: `new`, `register`, `register_initialization_failure`, `provider_for`,
/// `status`, and `statuses`.
/// Private members: the deterministic format-to-provider map.
#[derive(Clone, Default)]
pub struct CadProviderRegistry {
    providers: BTreeMap<CadFormat, ConfiguredProvider>,
}

/// Holds one initialized provider or one categorized initialization failure.
#[derive(Clone)]
enum ConfiguredProvider {
    /// Initialized provider responsible for the key format.
    Available {
        /// Provider handle available to host-side application operations.
        provider: Arc<dyn CadExternalProvider>,
        /// Validated immutable descriptor captured during registration.
        descriptor: CadProviderDescriptor,
    },
    /// Provider configured for the format but unable to initialize.
    Failed(CadProviderErrorKind),
}

impl Debug for CadProviderRegistry {
    /// Formats only provider-neutral status metadata.
    ///
    /// # Parameters
    /// - `self`: Registry being formatted.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatter status after writing safe provider statuses.
    ///
    /// # Errors
    /// Returns the formatter's error when writing the status fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CadProviderRegistry")
            .field("statuses", &self.statuses())
            .finish()
    }
}

/// Identifies a CAD provider registration conflict or invalid descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CadProviderRegistryError {
    /// Provider descriptor is missing a safe id, version, or format capability.
    InvalidDescriptor,
    /// Another provider or initialization failure already owns this format.
    FormatAlreadyRegistered(CadFormat),
}

impl CadProviderRegistry {
    /// Creates an empty registry for optional CAD providers.
    ///
    /// # Returns
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one initialized provider for each format in its descriptor.
    ///
    /// # Parameters
    /// - `self`: Registry being extended.
    /// - `provider`: Initialized provider implementation.
    ///
    /// # Returns
    /// The updated registry.
    ///
    /// # Errors
    /// Returns an error for an incomplete descriptor or a format already owned by
    /// another provider or failure record. Registration is atomic across formats.
    ///
    /// # Returns
    /// The registry after successful registration.
    pub fn register(
        mut self,
        provider: Arc<dyn CadExternalProvider>,
    ) -> Result<Self, CadProviderRegistryError> {
        let descriptor = provider.descriptor();
        validate_descriptor(&descriptor)?;
        for capability in &descriptor.formats {
            if self.providers.contains_key(&capability.format) {
                return Err(CadProviderRegistryError::FormatAlreadyRegistered(
                    capability.format.clone(),
                ));
            }
        }
        let provider_for_clones = Arc::clone(&provider);
        let mut provider = Some(provider);
        for capability in &descriptor.formats {
            self.providers.insert(
                capability.format.clone(),
                ConfiguredProvider::Available {
                    provider: provider
                        .take()
                        .unwrap_or_else(|| Arc::clone(&provider_for_clones)),
                    descriptor: descriptor.clone(),
                },
            );
        }
        Ok(self)
    }

    /// Records a configured provider that failed before exposing a descriptor.
    ///
    /// # Parameters
    /// - `self`: Registry being extended.
    /// - `format`: External CAD format assigned to the failed provider.
    ///
    /// # Returns
    /// The updated registry.
    ///
    /// # Errors
    /// Returns an error when the format already has a provider or failure record.
    pub fn register_initialization_failure(
        mut self,
        format: CadFormat,
    ) -> Result<Self, CadProviderRegistryError> {
        if self.providers.contains_key(&format) {
            return Err(CadProviderRegistryError::FormatAlreadyRegistered(format));
        }
        self.providers.insert(
            format,
            ConfiguredProvider::Failed(CadProviderErrorKind::InitializationFailed),
        );
        Ok(self)
    }

    /// Clones the initialized provider assigned to one exact external format.
    ///
    /// Failed initialization and absent formats both return `None`; callers can
    /// distinguish those states through [`Self::status`].
    ///
    /// # Parameters
    /// - `format`: Exact external format whose host-injected provider is requested.
    ///
    /// # Returns
    /// A shared provider handle when initialization succeeded for this format.
    #[must_use]
    pub fn provider_for(&self, format: &CadFormat) -> Option<Arc<dyn CadExternalProvider>> {
        match self.providers.get(format) {
            Some(ConfiguredProvider::Available { provider, .. }) => Some(Arc::clone(provider)),
            Some(ConfiguredProvider::Failed(_)) | None => None,
        }
    }

    /// Resolves one format to available, failed, or normal unavailable status.
    ///
    /// # Parameters
    /// - `self`: Registry to query.
    /// - `format`: Exact external CAD format to inspect.
    ///
    /// # Returns
    /// Provider-neutral status containing no provider-controlled diagnostic text.
    #[must_use]
    pub fn status(&self, format: &CadFormat) -> CadProviderStatus {
        match self.providers.get(format) {
            Some(ConfiguredProvider::Available { descriptor, .. }) => {
                let mut descriptor = descriptor.clone();
                descriptor
                    .formats
                    .retain(|capability| &capability.format == format);
                CadProviderStatus::Available(descriptor)
            }
            Some(ConfiguredProvider::Failed(kind)) => CadProviderStatus::Failed {
                format: format.clone(),
                kind: *kind,
            },
            None => CadProviderStatus::Unavailable(CadProviderUnavailable {
                format: format.clone(),
                reason: "no provider configured".to_owned(),
            }),
        }
    }

    /// Lists all configured format states and includes DWG when unconfigured.
    ///
    /// # Parameters
    /// - `self`: Registry to inspect.
    ///
    /// # Returns
    /// Stable format-ordered provider statuses with an explicit default DWG state.
    #[must_use]
    pub fn statuses(&self) -> Vec<CadProviderStatus> {
        let mut formats = self.providers.keys().cloned().collect::<Vec<_>>();
        if !formats.contains(&CadFormat::Dwg) {
            formats.push(CadFormat::Dwg);
        }
        formats.sort();
        formats.iter().map(|format| self.status(format)).collect()
    }
}

/// Validates safe provider identity and format capability metadata.
///
/// # Parameters
/// - `descriptor`: Provider metadata to validate before configuration.
///
/// # Returns
/// Unit when the descriptor can be exposed as safe capability metadata.
///
/// # Errors
/// Returns `InvalidDescriptor` when required safe metadata is missing, a
/// free-form identity token is unsafe, collections exceed their bounds, or
/// formats are duplicated.
fn validate_descriptor(descriptor: &CadProviderDescriptor) -> Result<(), CadProviderRegistryError> {
    if !safe_token(&descriptor.id)
        || !safe_token(&descriptor.version)
        || descriptor.formats.is_empty()
        || descriptor.formats.len() > 128
        || descriptor.deployment_requirements.len() > 128
    {
        return Err(CadProviderRegistryError::InvalidDescriptor);
    }
    let mut formats = Vec::<&CadFormat>::new();
    for capability in &descriptor.formats {
        let format = &capability.format;
        if formats.contains(&format)
            || matches!(format, CadFormat::Other(value) if !safe_token(value))
            || capability.supported_features.len() > 128
        {
            return Err(CadProviderRegistryError::InvalidDescriptor);
        }
        formats.push(format);
    }
    Ok(())
}

/// Checks a provider-controlled value before it crosses the capability boundary.
///
/// # Parameters
/// - `value`: Provider identifier/version, custom format, feature, or deployment token.
///
/// # Returns
/// `true` for a bounded token containing only visible safe identifier characters.
fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

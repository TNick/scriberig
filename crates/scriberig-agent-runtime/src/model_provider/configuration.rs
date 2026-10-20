//! Configures one normalized provider model adapter.

use crate::cache::{CachePolicy, ModelCache};
use crate::context::CompiledContext;
use crate::tools::ToolCatalog;
use scriberig_app::CapabilityContext;

use super::ProviderModel;

impl<P> ProviderModel<P> {
    /// Adds the trusted tool definitions exposed to future requests.
    ///
    /// - `self`: Model adapter being configured.
    /// - `tools`: Provider-neutral trusted tool definitions.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<crate::provider::ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Selects a provider-neutral thinking capability for future invocations.
    ///
    /// - `self`: Model adapter being configured.
    /// - `thinking_level`: Model-advertised capability identity.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_thinking_level(mut self, thinking_level: Option<String>) -> Self {
        self.thinking_level = thinking_level;
        self
    }

    /// Attaches the resolved provider identity to request provenance.
    ///
    /// - `self`: Model adapter being configured.
    /// - `provider_id`: Public upstream provider identity.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    /// Attaches an optional resolved provider identity to request provenance.
    ///
    /// - `self`: Model adapter being configured.
    /// - `provider_id`: Public upstream provider identity, when resolved.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_optional_provider_id(mut self, provider_id: Option<String>) -> Self {
        self.provider_id = provider_id;
        self
    }

    /// Attaches a `ScribeRig` provider profile to request provenance.
    ///
    /// - `self`: Model adapter being configured.
    /// - `provider_profile`: Safe local provider profile identifier.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_optional_provider_profile(mut self, provider_profile: Option<String>) -> Self {
        self.provider_profile = provider_profile;
        self
    }

    /// Attaches the trusted catalog used to validate returned tool calls.
    ///
    /// - `self`: Model adapter being configured.
    /// - `catalog`: Trusted branch-visible tool catalog.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_catalog(mut self, catalog: ToolCatalog) -> Self {
        self.tools = catalog.definitions();
        self.catalog = Some(catalog);
        self
    }

    /// Restricts provider-visible tools to this branch's capabilities.
    ///
    /// - `self`: Model adapter being configured.
    /// - `capabilities`: Authority available to this branch.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: CapabilityContext) -> Self {
        self.capabilities = capabilities;
        if let Some(catalog) = &self.catalog {
            self.tools = catalog
                .descriptors_for(&self.capabilities)
                .into_iter()
                .map(|descriptor| descriptor.definition)
                .collect();
        }
        self
    }

    /// Installs a complete-response cache for exact request reuse.
    ///
    /// - `self`: Model adapter being configured.
    /// - `cache`: Shared normalized response cache.
    /// - `policy`: Cache policy for provider invocations.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_cache(mut self, cache: ModelCache, policy: CachePolicy) -> Self {
        self.cache = Some((cache, policy));
        self
    }

    /// Adds an isolated branch namespace to cache identity.
    ///
    /// - `self`: Model adapter being configured.
    /// - `namespace`: Stable branch identity.
    ///
    /// # Returns
    /// The adapter with branch-scoped cache keys.
    #[must_use]
    pub fn with_cache_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.cache_namespace = Some(namespace.into());
        self
    }

    /// Adds the selected compiled context to each provider request.
    ///
    /// - `self`: Model adapter being configured.
    /// - `context`: Exact context selected for this branch.
    ///
    /// # Returns
    /// The adapter with provider-visible, cache-bound context.
    #[must_use]
    pub fn with_compiled_context(mut self, context: CompiledContext) -> Self {
        self.compiled_context = Some(context);
        self
    }
}

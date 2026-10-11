//! Provides deterministic discovery of installed domains and capabilities.

use std::collections::BTreeMap;

use crate::{CapabilityDescriptor, DomainDescriptor, DomainId};
use scriberig_core::ObjectType;

/// Reports a failed domain registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    /// A domain with the same stable identifier is already registered.
    DuplicateDomain {
        /// Identifier already present in the registry.
        id: DomainId,
    },
    /// A capability is advertised for an entity type the domain does not provide.
    UnknownEntityType {
        /// Domain containing the invalid capability.
        domain: DomainId,
        /// Entity type referenced by the invalid capability.
        entity_type: ObjectType,
    },
}

/// Stores installed domain descriptors without depending on concrete adapters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DomainRegistry {
    domains: BTreeMap<DomainId, DomainDescriptor>,
}

impl DomainRegistry {
    /// Creates an empty domain registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
        }
    }

    /// Registers one installed domain descriptor.
    ///
    /// # Parameters
    /// - `self`: Registry to update.
    /// - `descriptor`: Domain metadata to register.
    ///
    /// # Returns
    /// Unit after successful registration.
    ///
    /// # Errors
    /// Returns a duplicate or unknown-entity error when registration is invalid.
    pub fn register(&mut self, descriptor: DomainDescriptor) -> Result<(), RegistryError> {
        if self.domains.contains_key(&descriptor.id) {
            return Err(RegistryError::DuplicateDomain { id: descriptor.id });
        }
        for capability in &descriptor.capabilities {
            if !descriptor.entity_types.contains(&capability.entity_type) {
                return Err(RegistryError::UnknownEntityType {
                    domain: descriptor.id.clone(),
                    entity_type: capability.entity_type.clone(),
                });
            }
        }
        self.domains.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// Finds a domain by stable identifier.
    ///
    /// # Parameters
    /// - `self`: Registry to query.
    /// - `id`: Domain identifier to find.
    ///
    /// # Returns
    /// The registered descriptor, when present.
    #[must_use]
    pub fn domain(&self, id: &DomainId) -> Option<&DomainDescriptor> {
        self.domains.get(id)
    }

    /// Lists installed domains in deterministic identifier order.
    ///
    /// # Parameters
    /// - `self`: Registry to query.
    ///
    /// # Returns
    /// Registered domain descriptors.
    pub fn domains(&self) -> impl Iterator<Item = &DomainDescriptor> {
        self.domains.values()
    }

    /// Collects capabilities available for one entity type.
    ///
    /// # Parameters
    /// - `self`: Registry to query.
    /// - `entity_type`: Entity type whose capabilities are requested.
    ///
    /// # Returns
    /// Capabilities from installed and available providers in deterministic order.
    #[must_use]
    pub fn capabilities_for(&self, entity_type: &ObjectType) -> Vec<&CapabilityDescriptor> {
        let mut capabilities = self
            .domains
            .values()
            .flat_map(|domain| domain.capabilities.iter())
            .filter(|capability| {
                capability.entity_type == *entity_type
                    && capability
                        .provider
                        .as_ref()
                        .is_none_or(|provider| provider.available)
            })
            .collect::<Vec<_>>();
        capabilities.sort_by(|left, right| left.id.cmp(&right.id));
        capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::{DomainRegistry, RegistryError};
    use crate::{
        CapabilityCategory, CapabilityDescriptor, CapabilityId, DomainDescriptor, DomainId,
        ProviderMetadata,
    };
    use scriberig_core::ObjectType;

    /// Checks a registered domain can be found by stable identifier.
    #[test]
    fn domains_are_discoverable_by_id() {
        let domain = descriptor("markdown", "markdown.section", true);
        let id = domain.id.clone();
        let mut registry = DomainRegistry::new();
        registry
            .register(domain)
            .expect("registration should succeed");
        assert_eq!(
            registry.domain(&id).map(|item| item.name.as_str()),
            Some("Markdown")
        );
    }

    /// Checks optional providers do not advertise unavailable capabilities.
    #[test]
    fn unavailable_provider_is_not_advertised() {
        let mut registry = DomainRegistry::new();
        registry
            .register(descriptor("rust", "rust.function", false))
            .expect("registration should succeed");
        let entity_type = ObjectType::new("rust.function").expect("valid type");
        assert_eq!(registry.capabilities_for(&entity_type).len(), 0);
    }

    /// Checks a full-style registry exposes only its installed domains.
    #[test]
    fn full_style_registry_combines_installed_domains() {
        let mut registry = DomainRegistry::new();
        registry
            .register(descriptor("markdown", "markdown.section", true))
            .expect("Markdown registration should succeed");
        registry
            .register(descriptor("rust", "rust.function", true))
            .expect("Rust registration should succeed");
        let markdown = ObjectType::new("markdown.section").expect("valid type");
        let rust = ObjectType::new("rust.function").expect("valid type");
        assert_eq!(registry.capabilities_for(&markdown).len(), 1);
        assert_eq!(registry.capabilities_for(&rust).len(), 1);
    }

    /// Checks duplicate domains are rejected deterministically.
    #[test]
    fn duplicate_domains_are_rejected() {
        let mut registry = DomainRegistry::new();
        registry
            .register(descriptor("markdown", "markdown.section", true))
            .expect("first registration should succeed");
        let error = registry
            .register(descriptor("markdown", "markdown.section", true))
            .expect_err("duplicate should fail");
        assert!(matches!(error, RegistryError::DuplicateDomain { .. }));
    }

    /// Checks capability identifiers reject unsafe transport characters.
    #[test]
    fn capability_ids_are_transport_safe() {
        assert!(CapabilityId::new("projection.markdown").is_ok());
        assert!(CapabilityId::new("projection/markdown").is_err());
    }

    /// Builds a test domain descriptor with one capability.
    ///
    /// # Parameters
    /// - `domain`: Stable domain identifier.
    /// - `entity_type`: Entity type advertised by the domain.
    /// - `available`: Provider availability flag.
    ///
    /// # Returns
    /// A deterministic domain descriptor.
    fn descriptor(domain: &str, entity_type: &str, available: bool) -> DomainDescriptor {
        DomainDescriptor {
            id: DomainId::new(domain).expect("valid domain"),
            name: "Markdown".to_owned(),
            schema_version: 1.into(),
            entity_types: vec![ObjectType::new(entity_type).expect("valid type")],
            relation_types: Vec::new(),
            capabilities: vec![CapabilityDescriptor {
                id: CapabilityId::new("inspect.summary").expect("valid capability"),
                entity_type: ObjectType::new(entity_type).expect("valid type"),
                category: CapabilityCategory::Description,
                description: "Inspect summary".to_owned(),
                provider: Some(ProviderMetadata {
                    id: domain.to_owned(),
                    version: "1".to_owned(),
                    profile: None,
                    available,
                }),
            }],
        }
    }
}

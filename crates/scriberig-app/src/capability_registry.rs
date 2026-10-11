//! Composes injected domain capability providers deterministically.

use crate::{ApplicationError, CapabilityDescriptor, CapabilityId, DomainOperationRequest};
use scriberig_query::{Inspection, ReadCapability};

/// Supplies capability descriptors and optional domain operation execution.
pub trait CapabilityProvider {
    /// Returns descriptors advertised by this provider.
    ///
    /// # Returns
    /// Capability descriptors for the provider's installed domain behavior.
    fn capabilities(&self) -> Vec<CapabilityDescriptor>;

    /// Executes one advertised domain operation.
    ///
    /// # Parameters
    /// - `self`: Provider owning the capability implementation.
    /// - `request`: Exact semantic subject and change context.
    ///
    /// # Returns
    /// A canonical change handle produced by the provider.
    ///
    /// # Errors
    /// Returns a structured application failure when the capability is absent
    /// or unavailable.
    fn execute(
        &self,
        request: &DomainOperationRequest,
    ) -> Result<CapabilityExecution, ApplicationError> {
        Err(ApplicationError::UnsupportedCapability {
            operation: "domain operation".to_owned(),
            capability: request.capability.to_string(),
        })
    }
}

/// Describes the canonical change handoff from a domain operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityExecution {
    /// Stable change or migration handle for application-level application.
    pub change: String,
}

/// Reports a duplicate capability registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateCapability {
    /// Stable duplicate capability identifier.
    pub id: CapabilityId,
    /// Entity-type applicability shared by the duplicate descriptors.
    pub entity_types: Vec<scriberig_core::ObjectType>,
}

/// Rejects duplicate capability registrations while composing providers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityRegistrationError {
    /// Two providers claim the same identifier and applicability.
    Duplicate(DuplicateCapability),
}

/// Injected registry for generic and domain-provided capabilities.
#[derive(Default)]
pub struct CapabilityRegistry {
    providers: Vec<Box<dyn CapabilityProvider>>,
}

impl CapabilityRegistry {
    /// Creates an empty capability registry.
    ///
    /// # Returns
    /// An empty injected registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers one capability provider after checking duplicate claims.
    ///
    /// # Parameters
    /// - `self`: Registry being extended.
    /// - `provider`: Domain or generic capability provider.
    ///
    /// # Returns
    /// Unit when every descriptor is uniquely applicable.
    ///
    /// # Errors
    /// Returns the first deterministic duplicate descriptor.
    pub fn register<P: CapabilityProvider + 'static>(
        &mut self,
        provider: P,
    ) -> Result<(), CapabilityRegistrationError> {
        let incoming = provider.capabilities();
        let existing = self.capabilities();
        for candidate in &incoming {
            if existing.iter().any(|registered| {
                registered.id == candidate.id && registered.entity_types == candidate.entity_types
            }) {
                return Err(CapabilityRegistrationError::Duplicate(
                    DuplicateCapability {
                        id: candidate.id.clone(),
                        entity_types: candidate.entity_types.clone(),
                    },
                ));
            }
        }
        self.providers.push(Box::new(provider));
        Ok(())
    }

    /// Returns all available and unavailable descriptors in stable order.
    ///
    /// # Parameters
    /// - `self`: Registry to inspect.
    ///
    /// # Returns
    /// Deterministically ordered capability descriptors.
    #[must_use]
    pub fn capabilities(&self) -> Vec<CapabilityDescriptor> {
        let mut capabilities = self
            .providers
            .iter()
            .flat_map(|provider| provider.capabilities())
            .collect::<Vec<_>>();
        capabilities.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then(left.entity_types.cmp(&right.entity_types))
        });
        capabilities
    }

    /// Merges applicable registered capabilities into one inspection.
    ///
    /// # Parameters
    /// - `self`: Registry containing generic and domain providers.
    /// - `inspection`: Inspection to enrich in place.
    ///
    /// # Effects
    /// Only available descriptors applicable to the inspected entity type are
    /// added. Existing graph-provided descriptors are preserved and not
    /// duplicated by identifier.
    pub fn enrich_inspection(&self, inspection: &mut Inspection) {
        let entity_type = inspection.entity_type.as_ref();
        for descriptor in self.capabilities() {
            if !descriptor.available
                || (!descriptor.entity_types.is_empty()
                    && !entity_type.is_some_and(|value| {
                        descriptor
                            .entity_types
                            .iter()
                            .any(|candidate| candidate == value)
                    }))
            {
                continue;
            }
            let target = match descriptor.kind {
                crate::CapabilityKind::Relation => &mut inspection.relations,
                crate::CapabilityKind::Collection => &mut inspection.collections,
                crate::CapabilityKind::View => &mut inspection.views,
                crate::CapabilityKind::DomainOperation => &mut inspection.operations,
                crate::CapabilityKind::FindStrategy => continue,
            };
            if target
                .iter()
                .any(|value| value.id == descriptor.id.as_str())
            {
                continue;
            }
            target.push(ReadCapability {
                id: descriptor.id.to_string(),
                description: descriptor.description,
            });
        }
    }

    /// Executes a matching capability through its owning provider.
    ///
    /// # Parameters
    /// - `self`: Registry containing providers.
    /// - `request`: Domain operation request.
    ///
    /// # Returns
    /// The provider's canonical change handoff.
    ///
    /// # Errors
    /// Returns unsupported capability or provider execution failure.
    pub fn execute(
        &self,
        request: &DomainOperationRequest,
    ) -> Result<CapabilityExecution, ApplicationError> {
        for provider in &self.providers {
            if provider
                .capabilities()
                .iter()
                .any(|descriptor| descriptor.id == request.capability && descriptor.available)
            {
                return provider.execute(request);
            }
        }
        Err(ApplicationError::UnsupportedCapability {
            operation: "domain operation".to_owned(),
            capability: request.capability.to_string(),
        })
    }
}

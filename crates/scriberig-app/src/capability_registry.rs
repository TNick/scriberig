//! Composes injected domain capability providers deterministically.

use crate::{
    ApplicationContext, ApplicationError, CapabilityAction, CapabilityDescriptor, CapabilityId,
    CapabilityScope, DomainOperationRequest, OperationKind,
};
use scriberig_core::ObjectType;
use scriberig_query::{
    DetailLevel, EntityAtRevision, EntityInSnapshot, InspectRequest, Inspection, ReadCapability,
    TargetRef,
};

/// Builds inspection of the exact semantic subject before domain dispatch.
///
/// - `request`: Domain operation whose selected revision or snapshot is exact.
///
/// A compact inspection request bound to the operation subject.
/// # Returns
/// The operation result.
pub(crate) fn subject_inspection(request: &DomainOperationRequest) -> InspectRequest {
    let target = request.subject.revision.map_or_else(
        || {
            TargetRef::Snapshot(EntityInSnapshot {
                entity: request.subject.entity,
                snapshot: request.subject.snapshot,
            })
        },
        |revision| {
            TargetRef::Revision(EntityAtRevision {
                entity: request.subject.entity,
                revision,
            })
        },
    );
    InspectRequest {
        target,
        detail: DetailLevel::Compact,
        view: None,
    }
}

/// Supplies capability descriptors and optional domain operation execution.
pub trait CapabilityProvider {
    /// Returns descriptors advertised by this provider.
    ///
    /// Capability descriptors for the provider's installed domain behavior.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<CapabilityDescriptor>;

    /// Executes one advertised domain operation.
    ///
    /// - `self`: Provider owning the capability implementation.
    /// - `request`: Exact semantic subject and change context.
    ///
    /// A canonical change handle produced by the provider.
    ///
    /// Returns a structured application failure when the capability is absent
    /// or unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// An empty injected registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers one capability provider after checking duplicate claims.
    ///
    /// - `self`: Registry being extended.
    /// - `provider`: Domain or generic capability provider.
    ///
    /// Unit when every descriptor is uniquely applicable.
    ///
    /// # Returns
    /// Unit after all provider claims are registered.
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
    /// - `self`: Registry to inspect.
    ///
    /// Deterministically ordered capability descriptors.
    #[must_use]
    /// # Returns
    /// The operation result.
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

    /// Executes a capability after checking its selected entity type.
    ///
    /// - `self`: Registry containing providers.
    /// - `context`: Trusted caller capabilities and snapshot.
    /// - `request`: Exact semantic operation request.
    /// - `entity_type`: Type resolved from the exact selected snapshot.
    ///
    /// The provider's canonical change handoff.
    ///
    /// Returns authorization, applicability, or provider failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn execute_for_entity_type(
        &self,
        context: &ApplicationContext,
        request: &DomainOperationRequest,
        entity_type: Option<&ObjectType>,
    ) -> Result<CapabilityExecution, ApplicationError> {
        Self::authorize(context, request)?;
        for provider in &self.providers {
            if provider.capabilities().iter().any(|descriptor| {
                descriptor.id == request.capability
                    && descriptor.kind == crate::CapabilityKind::DomainOperation
                    && descriptor.available
                    && (descriptor.entity_types.is_empty()
                        || entity_type.is_some_and(|entity_type| {
                            descriptor.entity_types.contains(entity_type)
                        }))
            }) {
                return provider.execute(request);
            }
        }
        Err(ApplicationError::UnsupportedCapability {
            operation: "domain operation".to_owned(),
            capability: request.capability.to_string(),
        })
    }

    /// Checks exact snapshot binding and scoped semantic-operation authority.
    ///
    /// - `context`: Trusted caller capabilities and snapshot.
    /// - `request`: Exact semantic operation request.
    ///
    /// Unit when the request is authorized.
    ///
    /// Returns a stale-snapshot, invalid-request, or permission error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn authorize(
        context: &ApplicationContext,
        request: &DomainOperationRequest,
    ) -> Result<(), ApplicationError> {
        if request.subject.snapshot != request.context.base_snapshot {
            return Err(ApplicationError::InvalidRequest {
                operation: "domain operation".to_owned(),
                message: "subject and candidate base snapshots must match".to_owned(),
            });
        }
        if context
            .snapshot
            .is_some_and(|snapshot| snapshot != request.context.base_snapshot)
        {
            return Err(ApplicationError::StaleBaseSnapshot {
                requested: request.context.base_snapshot,
                current: context.snapshot.unwrap_or(request.context.base_snapshot),
            });
        }
        let action = CapabilityAction::Custom(request.capability.to_string());
        let generic_action = CapabilityAction::Custom("domain_operation".to_owned());
        let scope = CapabilityScope::Entity(request.subject.entity);
        let named_grant = context.capabilities.allows(&request.capability)
            || context
                .capabilities
                .granted
                .iter()
                .any(|capability| capability.as_str() == "domain_operation");
        let exact_grant = context.capabilities.permits(
            &action,
            &scope,
            Some(request.context.base_snapshot),
            Some(OperationKind::DomainOperation),
        );
        let generic_grant = context.capabilities.permits(
            &generic_action,
            &scope,
            Some(request.context.base_snapshot),
            Some(OperationKind::DomainOperation),
        );
        if !named_grant && !exact_grant && !generic_grant {
            return Err(ApplicationError::PermissionDenied {
                action: format!("{action:?}"),
                scope: format!("{scope:?}"),
            });
        }
        Ok(())
    }
}

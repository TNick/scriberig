//! Verifies injected domain capability composition.

use scriberig_app::{
    CapabilityDescriptor, CapabilityId, CapabilityKind, CapabilityProvider,
    CapabilityRegistrationError, CapabilityRegistry,
};
use scriberig_core::ObjectType;

/// Provides one deterministic domain capability.
struct Provider {
    id: &'static str,
}

impl CapabilityProvider for Provider {
    /// Returns the provider's one capability descriptor.
    ///
    /// One available capability for the test entity type.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<CapabilityDescriptor> {
        vec![CapabilityDescriptor {
            id: CapabilityId::new(self.id).expect("capability"),
            kind: CapabilityKind::DomainOperation,
            label: self.id.to_owned(),
            description: "test capability".to_owned(),
            entity_types: vec![ObjectType::new("test.entity").expect("type")],
            available: true,
            derived: false,
            estimated_items: None,
        }]
    }
}

/// Verifies independent providers compose and duplicate claims are rejected.
#[test]
fn registry_composes_and_rejects_duplicates() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(Provider { id: "domain.a" })
        .expect("first");
    registry
        .register(Provider { id: "domain.b" })
        .expect("second");
    assert_eq!(registry.capabilities().len(), 2);
    let duplicate = registry
        .register(Provider { id: "domain.a" })
        .expect_err("duplicate");
    assert!(matches!(
        duplicate,
        CapabilityRegistrationError::Duplicate(_)
    ));
}

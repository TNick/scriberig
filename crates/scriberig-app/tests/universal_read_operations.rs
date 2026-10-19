//! Verifies universal application dispatch.

#[path = "universal_operation_support.rs"]
mod support;
use scriberig_app::*;
use scriberig_core::*;
use scriberig_query::*;
use support::*;

/// Advertises a domain operation for an incompatible semantic type.
struct WrongTypeCapabilityProvider;

impl CapabilityProvider for WrongTypeCapabilityProvider {
    /// Returns an operation that applies only to another object type.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<CapabilityDescriptor> {
        vec![CapabilityDescriptor {
            id: CapabilityId::new("operation.wrong_type").expect("capability"),
            kind: CapabilityKind::DomainOperation,
            label: "Wrong type".to_owned(),
            description: "Must not execute on test.entity".to_owned(),
            entity_types: vec![ObjectType::new("other.entity").expect("type")],
            available: true,
            derived: false,
            estimated_items: None,
        }]
    }
}
/// Verifies every universal read variant reaches its typed provider.
#[test]
fn read_dispatch_preserves_structured_unsupported_capability() {
    let snapshot = SnapshotId::new([2; 32]);
    let entity = scriberig_core::EntityId::from_bytes([3; 16]);
    let subject = scriberig_query::EntityInSnapshot { entity, snapshot };
    let backend = CompositeBackend::new(ReadFixture, OperationFixture::new());
    let context = ApplicationContext::empty();
    let requests = [
        ApplicationRequest::Inspect(scriberig_query::InspectRequest {
            target: scriberig_query::TargetRef::Snapshot(subject),
            detail: scriberig_query::DetailLevel::Identity,
            view: None,
        }),
        ApplicationRequest::List(ListRequest {
            subject,
            collection: scriberig_query::CollectionRef {
                owner: entity,
                collection_type: "children".to_owned(),
            },
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Follow(FollowRequest {
            subject,
            relation: scriberig_query::RelationRef {
                source: entity,
                relation_type: "calls".to_owned(),
                target: None,
            },
            direction: scriberig_query::FollowDirection::Outgoing,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Find(FindRequest {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            query: "x".to_owned(),
            strategy: scriberig_query::FindStrategy::ExactText,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
        ApplicationRequest::Query(QuerySpec {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            expression: scriberig_query::QueryExpr::All,
            ordering: scriberig_query::QueryOrdering::StableDefault,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Identity,
        }),
    ];
    for request in requests {
        assert!(matches!(
            scriberig_app::ApplicationBackend::execute(&backend, &context, &request),
            Err(scriberig_app::ApplicationError::UnsupportedCapability { .. })
        ));
    }
}

/// Verifies registered capabilities are merged into universal inspection.
#[test]
fn inspection_merges_registered_capabilities() {
    let snapshot = SnapshotId::new([2; 32]);
    let entity = scriberig_core::EntityId::from_bytes([3; 16]);
    let mut registry = scriberig_app::CapabilityRegistry::new();
    registry
        .register(InspectionCapabilityProvider)
        .expect("register capability provider");
    let backend = CompositeBackend::new(InspectingReadFixture, OperationFixture::new())
        .with_capability_registry(registry);
    let request = ApplicationRequest::Inspect(scriberig_query::InspectRequest {
        target: scriberig_query::TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
            entity,
            snapshot,
        }),
        detail: scriberig_query::DetailLevel::Compact,
        view: None,
    });

    let result = scriberig_app::ApplicationBackend::execute(
        &backend,
        &ApplicationContext::empty(),
        &request,
    )
    .expect("inspection");
    let ApplicationResult::Inspect(inspection) = result else {
        panic!("expected inspection result");
    };
    assert!(
        inspection
            .relations
            .iter()
            .any(|value| value.id == "relation.calls")
    );
    assert!(
        inspection
            .views
            .iter()
            .any(|value| value.id == "view.summary")
    );
    assert!(
        inspection
            .operations
            .iter()
            .any(|value| value.id == "operation.rename")
    );
}

/// Executes an authorized domain operation and hands its candidate to Apply.
#[test]
fn domain_operation_uses_registered_provider_and_normal_change_path() {
    let snapshot = SnapshotId::new([8; 32]);
    let entity = EntityId::from_bytes([9; 16]);
    let mut registry = CapabilityRegistry::new();
    registry
        .register(InspectionCapabilityProvider)
        .expect("register domain provider");
    let backend = CompositeBackend::new(InspectingReadFixture, OperationFixture::new())
        .with_capability_registry(registry);
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(entity),
            actions: [CapabilityAction::Custom("operation.rename".to_owned())]
                .into_iter()
                .collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::empty()
    };
    let request = DomainOperationRequest {
        capability: CapabilityId::new("operation.rename").expect("capability"),
        subject: Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        },
        context: BranchContext {
            base_snapshot: snapshot,
            branch: None,
        },
        arguments: vec![("name".to_owned(), QueryValue::Text("renamed".to_owned()))],
    };

    let result = ApplicationBackend::execute(
        &backend,
        &context,
        &ApplicationRequest::DomainOperation(request),
    )
    .expect("registered operation");
    let ApplicationResult::DomainOperation(candidate) = result else {
        panic!("expected candidate change handle");
    };
    assert!(candidate.change.starts_with("candidate:"));
    let applied = ApplicationBackend::execute(
        &backend,
        &context,
        &ApplicationRequest::Apply(ApplyRequest {
            base_snapshot: snapshot,
            branch: None,
            change: candidate.change,
        }),
    )
    .expect("normal change application");
    assert!(matches!(applied, ApplicationResult::Apply(_)));
}

/// Rejects a domain operation when its exact entity grant is absent.
#[test]
fn domain_operation_requires_exact_scoped_authority() {
    let snapshot = SnapshotId::new([10; 32]);
    let entity = EntityId::from_bytes([11; 16]);
    let mut registry = CapabilityRegistry::new();
    registry
        .register(InspectionCapabilityProvider)
        .expect("register domain provider");
    let backend = CompositeBackend::new(InspectingReadFixture, OperationFixture::new())
        .with_capability_registry(registry);
    let request = ApplicationRequest::DomainOperation(DomainOperationRequest {
        capability: CapabilityId::new("operation.rename").expect("capability"),
        subject: Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        },
        context: BranchContext {
            base_snapshot: snapshot,
            branch: None,
        },
        arguments: Vec::new(),
    });

    assert!(matches!(
        ApplicationBackend::execute(&backend, &ApplicationContext::empty(), &request),
        Err(ApplicationError::PermissionDenied { .. })
    ));
}

/// Rejects a registered operation when the selected entity type differs.
#[test]
fn domain_operation_requires_descriptor_entity_type_match() {
    let snapshot = SnapshotId::new([12; 32]);
    let entity = EntityId::from_bytes([13; 16]);
    let mut registry = CapabilityRegistry::new();
    registry
        .register(WrongTypeCapabilityProvider)
        .expect("register domain provider");
    let backend = CompositeBackend::new(InspectingReadFixture, OperationFixture::new())
        .with_capability_registry(registry);
    let context = ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(entity),
            actions: [CapabilityAction::Custom("operation.wrong_type".to_owned())]
                .into_iter()
                .collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::empty()
    };
    let request = ApplicationRequest::DomainOperation(DomainOperationRequest {
        capability: CapabilityId::new("operation.wrong_type").expect("capability"),
        subject: Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        },
        context: BranchContext {
            base_snapshot: snapshot,
            branch: None,
        },
        arguments: Vec::new(),
    });

    assert!(matches!(
        ApplicationBackend::execute(&backend, &context, &request),
        Err(ApplicationError::UnsupportedCapability { .. })
    ));
}

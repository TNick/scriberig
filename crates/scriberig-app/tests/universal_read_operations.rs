//! Verifies universal application dispatch.

#[path = "universal_operation_support.rs"]
mod support;
use scriberig_app::*;
use scriberig_core::*;
use scriberig_query::*;
use support::*;
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

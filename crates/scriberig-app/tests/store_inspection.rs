//! Verifies inspection against the canonical in-memory storage contracts.

use std::collections::BTreeSet;

use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, GuardedQueryService, InspectRequest, QueryService,
    SnapshotResolution, StoreInspectionBackend,
};
use scriberig_core::{EncodingVersion, EntityId, ObjectType};
use scriberig_store::{
    Revision, RevisionStore, RootStore, SnapshotChange, SnapshotRelation, SnapshotRelationChange,
    SnapshotStore,
};
use scriberig_store_memory::MemoryStore;

/// Creates an application context with project-wide inspection authority.
///
/// A context sufficient for one trusted local inspection.
/// # Returns
/// The operation result.
fn context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: [CapabilityAction::Inspect]
                .into_iter()
                .collect::<BTreeSet<_>>(),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::empty()
    }
}

/// Verifies the local application service reads exact data from storage.
#[test]
fn store_backed_inspection_resolves_revision_and_snapshot() {
    let store = MemoryStore::new();
    let entity = EntityId::from_bytes([7; 16]);
    let referenced = EntityId::from_bytes([8; 16]);
    let revision = Revision::new(
        EncodingVersion::V1,
        ObjectType::new("test.entity").expect("valid type"),
        b"payload".to_vec(),
    )
    .expect("supported version");
    let revision_id = revision.id();
    store.revisions().put(revision).expect("revision");
    let snapshot = store
        .apply_changes(
            store.empty_snapshot(),
            &[
                SnapshotChange::Set(entity, revision_id),
                SnapshotChange::Set(referenced, revision_id),
            ],
        )
        .expect("snapshot");
    let snapshot = store
        .apply_relation_changes(
            snapshot,
            &[SnapshotRelationChange::Add(SnapshotRelation::new(
                entity,
                ObjectType::new("document.references").expect("relation type"),
                referenced,
            ))],
        )
        .expect("relation snapshot");
    store
        .compare_and_set_root(
            scriberig_core::RootName::new("main").expect("root"),
            None,
            snapshot,
        )
        .expect("root");

    let service =
        GuardedQueryService::new(ApplicationService::new(StoreInspectionBackend::new(store)));
    let result = service
        .inspect(
            &context(),
            &InspectRequest {
                entity,
                snapshot: SnapshotResolution::At(snapshot),
                detail: scriberig_app::DetailLevel::Compact,
            },
        )
        .expect("inspection");
    assert_eq!(result.entity, entity);
    assert_eq!(result.revision, Some(revision_id));
    assert_eq!(result.snapshot, Some(snapshot));
    assert!(
        result
            .capabilities
            .contains(&"document.references".to_owned())
    );
}

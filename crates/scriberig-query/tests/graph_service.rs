//! Verifies graph-backed inspect and traversal semantics.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, KnowledgeValue, ObjectType, RevisionId, SnapshotId};
use scriberig_graph::{
    CollectionValue, KnowledgeView, RelationValue, ReverseRelation, RevisionRecord,
};
use scriberig_query::{
    CollectionRef, DetailLevel, EntityInSnapshot, FindRequest, FindService, FindStrategy,
    FollowDirection, FollowRequest, GraphReadService, InspectRequest, InspectService, ListRequest,
    PageRequest, QueryExpr, QueryOrdering, QueryScope, QuerySpec, RelationRef,
    StructuredQueryService, TargetRef, TraverseService,
};

/// Provides one deterministic graph record to the read service.
struct Fixture {
    snapshot: SnapshotId,
    entity: EntityId,
    record: RevisionRecord,
    /// Optional cancellation signal raised after snapshot resolution.
    cancel_after_resolve: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Optional counter for observing revision reads.
    revision_reads: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
}

impl KnowledgeView for Fixture {
    type Error = &'static str;

    /// Lists the one fixture entity.
    ///
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot context.
    ///
    /// The fixture entity for its snapshot.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn entities(&self, snapshot: SnapshotId) -> Result<Vec<EntityId>, Self::Error> {
        Ok((snapshot == self.snapshot)
            .then_some(vec![self.entity])
            .unwrap_or_default())
    }

    /// Resolves the fixture entity at its exact snapshot.
    ///
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot to resolve.
    /// - `entity`: Entity to resolve.
    ///
    /// The fixture revision when the target matches.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, Self::Error> {
        if let Some(cancelled) = &self.cancel_after_resolve {
            cancelled.store(true, std::sync::atomic::Ordering::Release);
        }
        Ok(
            (snapshot == self.snapshot && entity == self.entity)
                .then_some(RevisionId::new([2; 32])),
        )
    }

    /// Reads the fixture revision at its exact snapshot.
    ///
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot to read.
    /// - `entity`: Entity to read.
    ///
    /// The fixture record when the target matches.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_revision(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionRecord>, Self::Error> {
        if let Some(reads) = &self.revision_reads {
            reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        Ok((snapshot == self.snapshot && entity == self.entity).then(|| self.record.clone()))
    }

    /// Reads the fixture revision by its immutable identity.
    ///
    /// - `entity`: Entity identity requested by the test.
    /// - `revision`: Revision identity requested by the test.
    ///
    /// The fixture record when both identities match.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_exact_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Result<Option<RevisionRecord>, Self::Error> {
        Ok(
            (entity == self.entity && revision == RevisionId::new([2; 32]))
                .then(|| self.record.clone()),
        )
    }

    /// Returns the fixture entity type.
    ///
    /// - `&self`: Fixture graph view.
    /// - `_snapshot`: Snapshot context.
    /// - `_entity`: Entity identity.
    ///
    /// The fixture entity type.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn entity_type(
        &self,
        _snapshot: SnapshotId,
        _entity: EntityId,
    ) -> Result<Option<ObjectType>, Self::Error> {
        Ok(Some(self.record.entity_type.clone()))
    }

    /// Returns no reverse relations for this fixture.
    ///
    /// - `&self`: Fixture graph view.
    /// - `_snapshot`: Snapshot context.
    /// - `_target`: Target entity.
    ///
    /// An empty reverse-relation list.
    ///
    /// This fixture does not produce backend errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn reverse_relations(
        &self,
        _snapshot: SnapshotId,
        _target: EntityId,
    ) -> Result<Vec<ReverseRelation>, Self::Error> {
        Ok(Vec::new())
    }
}

/// Builds a fixture containing one relation and one unordered collection.
///
/// A deterministic graph view fixture.
/// # Returns
/// The operation result.
fn fixture() -> Fixture {
    let snapshot = SnapshotId::new([3; 32]);
    let entity = EntityId::from_bytes([1; 16]);
    let relation_type = ObjectType::new("depends_on").expect("type");
    Fixture {
        snapshot,
        entity,
        record: RevisionRecord {
            entity,
            entity_type: ObjectType::new("test.entity").expect("type"),
            properties: BTreeMap::from([(
                "name".to_owned(),
                KnowledgeValue::String("one".to_owned()),
            )]),
            relations: BTreeMap::from([(
                "depends_on".to_owned(),
                RelationValue::new(relation_type, vec![EntityId::from_bytes([4; 16])]),
            )]),
            collections: BTreeMap::from([(
                "members".to_owned(),
                CollectionValue::Unordered(BTreeSet::from([EntityId::from_bytes([5; 16])])),
            )]),
        },
        cancel_after_resolve: None,
        revision_reads: None,
    }
}

/// Stops inspection between snapshot resolution and revision access.
#[test]
fn cancellable_inspection_stops_before_the_next_graph_read() {
    let mut fixture = fixture();
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let revision_reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    fixture.cancel_after_resolve = Some(std::sync::Arc::clone(&cancelled));
    fixture.revision_reads = Some(std::sync::Arc::clone(&revision_reads));
    let service = GraphReadService::new(fixture);
    let request = InspectRequest {
        target: TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
            entity: EntityId::from_bytes([1; 16]),
            snapshot: SnapshotId::new([3; 32]),
        }),
        detail: DetailLevel::Identity,
        view: None,
    };

    assert_eq!(
        service.inspect_cancellable(&request, &cancelled),
        Err(scriberig_query::QueryError::Cancelled {
            operation: scriberig_query::ReadOperation::Inspect,
        })
    );
    assert_eq!(revision_reads.load(std::sync::atomic::Ordering::Relaxed), 0);
}

/// Verifies inspection exposes graph capabilities and exact identity.
#[test]
fn graph_inspection_is_snapshot_bound() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let result = service
        .inspect(&InspectRequest {
            target: TargetRef::Snapshot(EntityInSnapshot {
                entity: EntityId::from_bytes([1; 16]),
                snapshot: SnapshotId::new([3; 32]),
            }),
            detail: DetailLevel::Compact,
            view: None,
        })
        .expect("inspection");
    assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
    assert_eq!(result.relations[0].id, "depends_on");
}

/// Verifies exact revision targets remain readable after snapshot changes.
#[test]
fn graph_inspection_resolves_exact_revision_targets() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let result = service
        .inspect(&InspectRequest {
            target: TargetRef::Revision(scriberig_query::EntityAtRevision {
                entity: EntityId::from_bytes([1; 16]),
                revision: RevisionId::new([2; 32]),
            }),
            detail: DetailLevel::Compact,
            view: None,
        })
        .expect("exact revision inspection");
    assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
    assert_eq!(result.snapshot, None);
}

/// Verifies collection and outgoing relation reads retain exact targets.
#[test]
fn graph_traversal_is_bounded_and_typed() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let subject = EntityInSnapshot {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotId::new([3; 32]),
    };
    let listed = service
        .list(&ListRequest {
            subject,
            collection: CollectionRef {
                owner: subject.entity,
                collection_type: "members".to_owned(),
            },
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("list");
    assert_eq!(listed.page.items.len(), 1);
    assert_eq!(
        listed.page.items[0].entity_type,
        Some(ObjectType::new("test.entity").expect("type"))
    );
    let followed = service
        .follow(&FollowRequest {
            subject,
            relation: RelationRef {
                source: subject.entity,
                relation_type: "depends_on".to_owned(),
                target: None,
            },
            direction: FollowDirection::Outgoing,
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("follow");
    assert_eq!(followed.page.items.len(), 1);

    let continuation = listed.page.next;
    assert!(continuation.is_none());
}

#[path = "graph_service/continuations.rs"]
mod continuations;

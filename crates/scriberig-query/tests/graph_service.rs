//! Verifies graph-backed inspect and traversal semantics.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, KnowledgeValue, ObjectType, RevisionId, SnapshotId};
use scriberig_graph::{
    CollectionValue, KnowledgeView, RelationValue, ReverseRelation, RevisionRecord,
};
use scriberig_query::{
    CollectionRef, DetailLevel, EntityInSnapshot, FollowDirection, FollowRequest, GraphReadService,
    InspectRequest, InspectService, ListRequest, PageRequest, RelationRef, TargetRef,
    TraverseService,
};

/// Provides one deterministic graph record to the read service.
struct Fixture {
    snapshot: SnapshotId,
    entity: EntityId,
    record: RevisionRecord,
}

impl KnowledgeView for Fixture {
    type Error = &'static str;

    /// Resolves the fixture entity at its exact snapshot.
    ///
    /// # Parameters
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot to resolve.
    /// - `entity`: Entity to resolve.
    ///
    /// # Returns
    /// The fixture revision when the target matches.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, Self::Error> {
        Ok(
            (snapshot == self.snapshot && entity == self.entity)
                .then_some(RevisionId::new([2; 32])),
        )
    }

    /// Reads the fixture revision at its exact snapshot.
    ///
    /// # Parameters
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot to read.
    /// - `entity`: Entity to read.
    ///
    /// # Returns
    /// The fixture record when the target matches.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
    fn read_revision(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionRecord>, Self::Error> {
        Ok((snapshot == self.snapshot && entity == self.entity).then(|| self.record.clone()))
    }

    /// Returns the fixture entity type.
    ///
    /// # Parameters
    /// - `&self`: Fixture graph view.
    /// - `_snapshot`: Snapshot context.
    /// - `_entity`: Entity identity.
    ///
    /// # Returns
    /// The fixture entity type.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
    fn entity_type(
        &self,
        _snapshot: SnapshotId,
        _entity: EntityId,
    ) -> Result<Option<ObjectType>, Self::Error> {
        Ok(Some(self.record.entity_type.clone()))
    }

    /// Returns no reverse relations for this fixture.
    ///
    /// # Parameters
    /// - `&self`: Fixture graph view.
    /// - `_snapshot`: Snapshot context.
    /// - `_target`: Target entity.
    ///
    /// # Returns
    /// An empty reverse-relation list.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
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
/// # Returns
/// A deterministic graph view fixture.
fn fixture() -> Fixture {
    let snapshot = SnapshotId::new([3; 32]);
    let entity = EntityId::new([1; 16]);
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
                RelationValue::new(relation_type, vec![EntityId::new([4; 16])]),
            )]),
            collections: BTreeMap::from([(
                "members".to_owned(),
                CollectionValue::Unordered(BTreeSet::from([EntityId::new([5; 16])])),
            )]),
        },
    }
}

/// Verifies inspection exposes graph capabilities and exact identity.
#[test]
fn graph_inspection_is_snapshot_bound() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let result = service
        .inspect(&InspectRequest {
            target: TargetRef::Snapshot(EntityInSnapshot {
                entity: EntityId::new([1; 16]),
                snapshot: SnapshotId::new([3; 32]),
            }),
            detail: DetailLevel::Compact,
            view: None,
        })
        .expect("inspection");
    assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
    assert_eq!(result.relations[0].id, "depends_on");
}

/// Verifies collection and outgoing relation reads retain exact targets.
#[test]
fn graph_traversal_is_bounded_and_typed() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let subject = EntityInSnapshot {
        entity: EntityId::new([1; 16]),
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
}

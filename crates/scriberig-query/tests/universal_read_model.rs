//! Verifies the reusable universal read model against an in-memory graph.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, KnowledgeValue, ObjectType, RevisionId, SnapshotId};
use scriberig_graph::{
    CollectionValue, KnowledgeView, RelationValue, ReverseRelation, RevisionRecord,
};
use scriberig_query::{
    CollectionRef, DetailLevel, EntityInSnapshot, FindRequest, FindService, FindStrategy,
    FollowDirection, FollowRequest, InspectRequest, InspectService, ListRequest, PageRequest,
    QueryExpr, QueryOrdering, QueryScope, QuerySpec, RelationRef, StructuredQueryService,
    TargetRef, TraverseService,
};

/// Provides a small multi-entity graph at one immutable snapshot.
struct Fixture {
    snapshot: SnapshotId,
    records: BTreeMap<EntityId, RevisionRecord>,
}

impl KnowledgeView for Fixture {
    type Error = &'static str;

    /// Returns every entity present in the fixture snapshot.
    /// - `snapshot`: Argument named `snapshot`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn entities(&self, snapshot: SnapshotId) -> Result<Vec<EntityId>, Self::Error> {
        Ok(if snapshot == self.snapshot {
            self.records.keys().copied().collect()
        } else {
            Vec::new()
        })
    }

    /// Resolves one entity to the fixture's immutable revision.
    /// - `snapshot`: Argument named `snapshot`.
    /// - `entity`: Argument named `entity`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, Self::Error> {
        Ok(
            (snapshot == self.snapshot && self.records.contains_key(&entity))
                .then_some(RevisionId::new([2; 32])),
        )
    }

    /// Reads one entity revision in the fixture snapshot.
    /// - `snapshot`: Argument named `snapshot`.
    /// - `entity`: Argument named `entity`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn read_revision(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionRecord>, Self::Error> {
        Ok((snapshot == self.snapshot)
            .then(|| self.records.get(&entity).cloned())
            .flatten())
    }

    /// Reads any fixture entity at its immutable revision.
    /// - `entity`: Argument named `entity`.
    /// - `revision`: Argument named `revision`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn read_exact_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Result<Option<RevisionRecord>, Self::Error> {
        Ok((revision == RevisionId::new([2; 32]))
            .then(|| self.records.get(&entity).cloned())
            .flatten())
    }

    /// Returns the semantic type of one fixture entity.
    /// - `_snapshot`: Argument named `_snapshot`.
    /// - `entity`: Argument named `entity`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn entity_type(
        &self,
        _snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<ObjectType>, Self::Error> {
        Ok(self
            .records
            .get(&entity)
            .map(|record| record.entity_type.clone()))
    }

    /// Deliberately omits reverse indexes to verify structured degradation.
    /// - `_snapshot`: Argument named `_snapshot`.
    /// - `_target`: Argument named `_target`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn reverse_relations(
        &self,
        _snapshot: SnapshotId,
        _target: EntityId,
    ) -> Result<Vec<ReverseRelation>, Self::Error> {
        Err("reverse index unavailable")
    }
}

/// Creates a project with an ordered collection and a typed relation.
///
/// # Returns
/// The value produced by the declaration.
fn fixture() -> Fixture {
    let snapshot = SnapshotId::new([9; 32]);
    let project = EntityId::from_bytes([1; 16]);
    let first = EntityId::from_bytes([2; 16]);
    let second = EntityId::from_bytes([3; 16]);
    let function_type = ObjectType::new("test.function").expect("type");
    let mut records = BTreeMap::new();
    records.insert(
        project,
        RevisionRecord {
            entity: project,
            entity_type: ObjectType::new("test.project").expect("type"),
            properties: BTreeMap::new(),
            relations: BTreeMap::from([(
                "calls".to_owned(),
                RelationValue::new(function_type.clone(), vec![first]),
            )]),
            collections: BTreeMap::from([(
                "modules".to_owned(),
                CollectionValue::Ordered(vec![first, second]),
            )]),
        },
    );
    for (entity, name) in [(first, "first"), (second, "second")] {
        records.insert(
            entity,
            RevisionRecord {
                entity,
                entity_type: function_type.clone(),
                properties: BTreeMap::from([(
                    "name".to_owned(),
                    KnowledgeValue::String(name.to_owned()),
                )]),
                relations: BTreeMap::new(),
                collections: BTreeMap::new(),
            },
        );
    }
    Fixture { snapshot, records }
}

/// Verifies inspection resolves an exact snapshot and advertises structure.
#[test]
fn inspection_is_snapshot_bound_and_progressively_disclosed() {
    let fixture = fixture();
    let service = scriberig_query::GraphReadService::new(fixture);
    let result = service
        .inspect(&InspectRequest {
            target: TargetRef::Snapshot(EntityInSnapshot {
                entity: EntityId::from_bytes([2; 16]),
                snapshot: SnapshotId::new([9; 32]),
            }),
            detail: DetailLevel::Compact,
            view: None,
        })
        .expect("inspection");
    assert_eq!(result.snapshot, Some(SnapshotId::new([9; 32])));
    assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
    assert_eq!(
        result.properties,
        vec![("name".to_owned(), "present".to_owned())]
    );
}

/// Verifies ordered collection pagination and handle composition.
#[test]
fn list_preserves_order_and_returns_reusable_handles() {
    let fixture = fixture();
    let service = scriberig_query::GraphReadService::new(fixture);
    let subject = EntityInSnapshot {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotId::new([9; 32]),
    };
    let request = |continuation| ListRequest {
        subject,
        collection: CollectionRef {
            owner: subject.entity,
            collection_type: "modules".to_owned(),
        },
        page: PageRequest::new(1, continuation).expect("page"),
        detail: DetailLevel::Compact,
    };
    let first = service.list(&request(None)).expect("first page");
    let second = service
        .list(&request(first.page.next.clone()))
        .expect("second page");
    assert_eq!(first.page.items[0].label.as_deref(), Some("first"));
    assert_eq!(second.page.items[0].label.as_deref(), Some("second"));
    let TargetRef::Snapshot(target) = first.page.items[0].target else {
        panic!("expected snapshot handle");
    };
    let inspected = service
        .inspect(&InspectRequest {
            target: TargetRef::Snapshot(target),
            detail: DetailLevel::Identity,
            view: None,
        })
        .expect("inspect listed handle");
    assert_eq!(
        inspected.entity_type,
        Some(ObjectType::new("test.function").expect("type"))
    );
}

/// Verifies typed relation identity and structured reverse-index failure.
#[test]
fn follow_preserves_relation_and_reports_missing_reverse_index() {
    let fixture = fixture();
    let service = scriberig_query::GraphReadService::new(fixture);
    let subject = EntityInSnapshot {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotId::new([9; 32]),
    };
    let request = FollowRequest {
        subject,
        relation: RelationRef {
            source: subject.entity,
            relation_type: "calls".to_owned(),
            target: None,
        },
        direction: FollowDirection::Outgoing,
        page: PageRequest::default(),
        detail: DetailLevel::Compact,
    };
    let result = service.follow(&request).expect("outgoing relation");
    assert_eq!(result.relation.relation_type, "calls");
    assert_eq!(result.page.items.len(), 1);
    let reverse = service
        .follow(&FollowRequest {
            direction: FollowDirection::Incoming,
            ..request
        })
        .expect_err("reverse index");
    assert!(matches!(
        reverse,
        scriberig_query::QueryError::DerivedDataUnavailable { .. }
    ));
}

/// Verifies structural predicates remain distinct from discovery.
#[test]
fn find_and_query_return_snapshot_bound_semantic_rows() {
    let fixture = fixture();
    let service = scriberig_query::GraphReadService::new(fixture);
    let snapshot = SnapshotId::new([9; 32]);
    let query = service
        .query(&QuerySpec {
            scope: QueryScope::Snapshot(snapshot),
            expression: QueryExpr::And(vec![
                QueryExpr::TypeIs(ObjectType::new("test.function").expect("type")),
                QueryExpr::PropertyExists {
                    key: "name".to_owned(),
                },
            ]),
            ordering: QueryOrdering::StableDefault,
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("query");
    assert_eq!(query.page.items.len(), 2);
    assert!(query.page.items.iter().all(|row| {
        matches!(row.target, TargetRef::Snapshot(EntityInSnapshot { snapshot: value, .. }) if value == snapshot)
    }));
    let find = service
        .find(&FindRequest {
            scope: QueryScope::ExplicitEntities {
                snapshot,
                entities: vec![EntityId::from_bytes([2; 16])],
            },
            query: "test.function".to_owned(),
            strategy: FindStrategy::ExactText,
            page: PageRequest::default(),
            detail: DetailLevel::Identity,
        })
        .expect("find");
    assert_eq!(find.page.items.len(), 1);
}

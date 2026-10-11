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
}

impl KnowledgeView for Fixture {
    type Error = &'static str;

    /// Lists the one fixture entity.
    ///
    /// # Parameters
    /// - `&self`: Fixture graph view.
    /// - `snapshot`: Snapshot context.
    ///
    /// # Returns
    /// The fixture entity for its snapshot.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
    fn entities(&self, snapshot: SnapshotId) -> Result<Vec<EntityId>, Self::Error> {
        Ok((snapshot == self.snapshot)
            .then_some(vec![self.entity])
            .unwrap_or_default())
    }

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

    /// Reads the fixture revision by its immutable identity.
    ///
    /// # Parameters
    /// - `entity`: Entity identity requested by the test.
    /// - `revision`: Revision identity requested by the test.
    ///
    /// # Returns
    /// The fixture record when both identities match.
    ///
    /// # Errors
    /// This fixture does not produce backend errors.
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

/// Verifies continuations cannot be replayed against a different operation.
#[test]
fn graph_traversal_rejects_cross_operation_continuations() {
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
            page: PageRequest::new(1, None).expect("page"),
            detail: DetailLevel::Compact,
        })
        .expect("list");
    let continuation = listed
        .page
        .next
        .unwrap_or_else(|| scriberig_query::ContinuationToken::new("0"));
    let error = service
        .follow(&FollowRequest {
            subject,
            relation: RelationRef {
                source: subject.entity,
                relation_type: "depends_on".to_owned(),
                target: None,
            },
            direction: FollowDirection::Outgoing,
            page: PageRequest::new(1, Some(continuation)).expect("page"),
            detail: DetailLevel::Compact,
        })
        .expect_err("cross-operation continuation");
    assert!(matches!(
        error,
        scriberig_query::QueryError::InvalidContinuation {
            operation: scriberig_query::ReadOperation::Follow,
        }
    ));
}

/// Verifies explicit entity scopes support discovery and structural queries.
#[test]
fn graph_explicit_scopes_support_find_and_query() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let snapshot = SnapshotId::new([3; 32]);
    let entity = EntityId::from_bytes([1; 16]);
    let find = service
        .find(&FindRequest {
            scope: QueryScope::ExplicitEntities {
                snapshot,
                entities: vec![entity],
            },
            query: "test.entity".to_owned(),
            strategy: FindStrategy::ExactText,
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("find");
    assert_eq!(find.page.items.len(), 1);
    let query = service
        .query(&QuerySpec {
            scope: QueryScope::Snapshot(snapshot),
            expression: QueryExpr::And(vec![
                QueryExpr::TypeIs(ObjectType::new("test.entity").expect("type")),
                QueryExpr::PropertyEq {
                    key: "name".to_owned(),
                    value: scriberig_query::QueryValue::Text("one".to_owned()),
                },
            ]),
            ordering: QueryOrdering::StableDefault,
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("query");
    assert_eq!(query.page.items.len(), 1);
}

/// Verifies invalid discovery inputs remain structured at the query boundary.
#[test]
fn graph_query_rejects_empty_scope_and_search_text() {
    let fixture = fixture();
    let service = GraphReadService::new(fixture);
    let snapshot = SnapshotId::new([3; 32]);
    let scope_error = service
        .query(&QuerySpec {
            scope: QueryScope::ExplicitEntities {
                snapshot,
                entities: Vec::new(),
            },
            expression: QueryExpr::All,
            ordering: QueryOrdering::StableDefault,
            page: PageRequest::default(),
            detail: DetailLevel::Identity,
        })
        .expect_err("empty scope");
    assert!(matches!(
        scope_error,
        scriberig_query::QueryError::InvalidScope {
            operation: scriberig_query::ReadOperation::Query,
            ..
        }
    ));

    let find_error = service
        .find(&FindRequest {
            scope: QueryScope::Snapshot(snapshot),
            query: " ".to_owned(),
            strategy: FindStrategy::ExactText,
            page: PageRequest::default(),
            detail: DetailLevel::Identity,
        })
        .expect_err("empty search text");
    assert!(matches!(
        find_error,
        scriberig_query::QueryError::InvalidQuery { .. }
    ));
}

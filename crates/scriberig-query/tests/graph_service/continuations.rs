//! Holds additional focused scenarios for the parent test module.

use super::*;

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

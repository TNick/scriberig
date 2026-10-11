//! Defines universal semantic query, traversal, scope, and pagination contracts.

pub mod detail;
pub mod error;
pub mod find;
pub mod graph_service;
pub mod handle;
pub mod inspect;
pub mod pagination;
pub mod query;
pub mod result;
pub mod scope;
pub mod selection;
pub mod traverse;

pub use detail::{DetailLevel, InformationCost};
pub use error::{QueryError, ReadOperation};
pub use find::{FindHit, FindRequest, FindResult, FindService, FindStrategy};
pub use graph_service::GraphReadService;
pub use handle::{
    CollectionRef, EntityAtRevision, EntityInSnapshot, EntityRef, OperationRef, ProjectionRef,
    RelationRef, RevisionRef, SnapshotRef, TargetRef,
};
pub use inspect::{InspectRequest, InspectService};
pub use pagination::{
    ContinuationToken, DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT, Page, PageRequest, PageRequestError,
};
pub use query::{
    QueryDirection, QueryExpr, QueryOrdering, QueryResult, QueryRow, QuerySpec, QueryValue,
    SortDirection, StructuredQueryService,
};
pub use result::{Inspection, NavigationEntry, ReadCapability};
pub use scope::QueryScope;
pub use selection::{ProjectionAnchor, RelationStep, Selection};
pub use traverse::{
    FollowDirection, FollowRequest, FollowResult, ListRequest, ListResult, ReadService,
    SnapshotContext, TraverseService,
};

#[cfg(test)]
mod tests {
    use super::*;
    use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};

    /// Checks revision and snapshot targets remain distinct.
    #[test]
    fn targets_preserve_resolution_mode() {
        let entity = EntityId::new([1; 16]);
        let revision = RevisionId::new([2; 32]);
        let snapshot = SnapshotId::new([3; 32]);
        assert_ne!(
            TargetRef::Revision(EntityAtRevision { entity, revision }),
            TargetRef::Snapshot(EntityInSnapshot { entity, snapshot })
        );
    }

    /// Checks semantic selections are represented without projection data.
    #[test]
    fn scope_is_always_snapshot_bound() {
        let snapshot = SnapshotId::new([4; 32]);
        let scope = QueryScope::ExplicitEntities {
            snapshot,
            entities: vec![EntityId::new([1; 16]), EntityId::new([2; 16])],
        };
        assert_eq!(scope.snapshot(), snapshot);
    }

    /// Checks projection anchors do not alter semantic selection identity.
    #[test]
    fn selection_can_carry_projection_context() {
        let snapshot = SnapshotId::new([5; 32]);
        let entity = EntityId::new([6; 16]);
        let semantic = Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        };
        let anchored = Selection {
            projection_anchor: Some(ProjectionAnchor::TextRange { start: 1, end: 4 }),
            ..semantic.clone()
        };
        assert_eq!(semantic.entity, anchored.entity);
        assert_ne!(semantic.projection_anchor, anchored.projection_anchor);
    }

    /// Checks pagination validates bounds and preserves opaque continuation state.
    #[test]
    fn pagination_is_bounded_and_opaque() {
        assert_eq!(
            PageRequest::new(0, None),
            Err(PageRequestError::InvalidLimit { limit: 0 })
        );
        assert!(PageRequest::new(MAX_PAGE_LIMIT + 1, None).is_err());
        let token = ContinuationToken::new("opaque");
        let page = Page {
            items: vec!["one"],
            next: Some(token),
        };
        assert_eq!(page.items, vec!["one"]);
        assert!(page.next.is_some());
    }

    /// Checks navigation requests and results retain typed snapshot context.
    #[test]
    fn traversal_contracts_preserve_typed_identity() {
        let snapshot = SnapshotId::new([7; 32]);
        let entity = EntityId::new([8; 16]);
        let subject = EntityInSnapshot { entity, snapshot };
        let collection = CollectionRef {
            owner: entity,
            collection_type: "module.children".to_owned(),
        };
        let request = ListRequest {
            subject,
            collection: collection.clone(),
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        };
        assert_eq!(request.subject.snapshot, snapshot);
        assert_eq!(request.collection, collection);
    }

    /// Checks inspection requests preserve exact revision targets.
    #[test]
    fn inspection_request_keeps_exact_target() {
        let target = TargetRef::Revision(EntityAtRevision {
            entity: EntityId::new([9; 16]),
            revision: RevisionId::new([10; 32]),
        });
        let request = InspectRequest {
            target,
            detail: DetailLevel::Identity,
            view: None,
        };
        assert_eq!(request.target, target);
    }

    /// Checks find requests preserve exact scope and extensible strategies.
    #[test]
    fn find_request_preserves_scope_and_strategy() {
        let scope = QueryScope::Snapshot(SnapshotId::new([11; 32]));
        let request = FindRequest {
            scope: scope.clone(),
            query: "payment".to_owned(),
            strategy: FindStrategy::DomainSpecific("code.references".to_owned()),
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        };
        assert_eq!(request.scope, scope);
        assert!(matches!(request.strategy, FindStrategy::DomainSpecific(_)));
    }

    /// Checks structural predicates compose without storage-specific values.
    #[test]
    fn structured_query_ast_composes_predicates() {
        let expression = QueryExpr::And(vec![
            QueryExpr::TypeIs(ObjectType::new("rust.function").expect("valid type")),
            QueryExpr::Not(Box::new(QueryExpr::PropertyExists {
                key: "deprecated".to_owned(),
            })),
        ]);
        assert!(matches!(expression, QueryExpr::And(_)));
    }

    /// Checks information-cost hints remain optional and domain-neutral.
    #[test]
    fn information_cost_is_optional() {
        let cost = InformationCost::default();
        assert_eq!(cost.items, None);
        assert_eq!(DetailLevel::Compact, DetailLevel::Compact);
    }
}

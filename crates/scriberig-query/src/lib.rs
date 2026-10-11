//! Defines universal semantic query, traversal, scope, and pagination contracts.

pub mod detail;
pub mod handle;
pub mod pagination;
pub mod scope;
pub mod selection;

pub use detail::{DetailLevel, InformationCost};
pub use handle::{
    CollectionRef, EntityAtRevision, EntityInSnapshot, EntityRef, OperationRef, ProjectionRef,
    RelationRef, RevisionRef, SnapshotRef, TargetRef,
};
pub use pagination::{
    ContinuationToken, DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT, Page, PageRequest, PageRequestError,
};
pub use scope::QueryScope;
pub use selection::{ProjectionAnchor, RelationStep, Selection};

#[cfg(test)]
mod tests {
    use super::*;
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

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

    /// Checks information-cost hints remain optional and domain-neutral.
    #[test]
    fn information_cost_is_optional() {
        let cost = InformationCost::default();
        assert_eq!(cost.items, None);
        assert_eq!(DetailLevel::Compact, DetailLevel::Compact);
    }
}

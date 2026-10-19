//! Verifies application read authorization at traversal and discovery boundaries.

use scriberig_app::{
    ApplicationContext, ApplicationError, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, GuardedQueryService,
};
use scriberig_core::{EntityId, ObjectType, SnapshotId};
use scriberig_query::{
    FindHit, FindRequest, FindResult, FindService, FindStrategy, FollowDirection, FollowRequest,
    FollowResult, InspectRequest, InspectService, Inspection, NavigationEntry, Page, PageRequest,
    ProjectionRef, QueryError, QueryScope, TargetRef, TraverseService,
};
use std::collections::BTreeSet;

const SNAPSHOT: SnapshotId = SnapshotId::new([7; 32]);
const ROOT: EntityId = EntityId::from_bytes([1; 16]);
const ALLOWED: EntityId = EntityId::from_bytes([2; 16]);
const RESTRICTED: EntityId = EntityId::from_bytes([3; 16]);

/// Creates one unconstrained capability grant for a test principal.
///
/// - `scope`: Scope covered by the grant.
/// - `action`: Action granted within that scope.
///
/// A test capability.
/// # Returns
/// The operation result.
fn grant(scope: CapabilityScope, action: CapabilityAction) -> Capability {
    Capability {
        scope,
        actions: BTreeSet::from([action]),
        constraints: CapabilityConstraints::unrestricted(),
    }
}

/// Creates an application context with exactly the supplied scoped grants.
///
/// - `grants`: Scoped grants supplied to the test context.
///
/// An application context containing those grants.
/// # Returns
/// The operation result.
fn context(grants: impl IntoIterator<Item = Capability>) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants(grants),
        ..ApplicationContext::default()
    }
}

/// Provides deterministic relation results containing one allowed and one hidden target.
struct TraversalFixture;

impl TraverseService for TraversalFixture {
    /// Returns a relation page for the guarded adapter test.
    ///
    /// - `request`: Relation request supplied by the guarded adapter.
    ///
    /// A deterministic page containing one allowed and one hidden target.
    ///
    /// This fixture returns no provider error for follow requests.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn follow(&self, request: &FollowRequest) -> Result<FollowResult, QueryError> {
        Ok(FollowResult {
            subject: request.subject,
            snapshot: SNAPSHOT,
            relation: request.relation.clone(),
            direction: FollowDirection::Outgoing,
            metadata: None,
            page: Page {
                items: vec![entry(ALLOWED), entry(RESTRICTED)],
                next: None,
            },
        })
    }

    /// The fixture does not need collection listing for this test.
    ///
    /// - `_request`: Collection request, intentionally unsupported.
    ///
    /// Never returns a successful listing.
    ///
    /// Always returns a structured unsupported-capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list(
        &self,
        _request: &scriberig_query::ListRequest,
    ) -> Result<scriberig_query::ListResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::List,
            capability: "fixture.list".to_owned(),
        })
    }
}

/// Provides deterministic find results for the guarded adapter test.
struct FindFixture;

impl FindService for FindFixture {
    /// Returns one allowed and one restricted discovery hit.
    ///
    /// - `request`: Discovery request supplied by the guarded adapter.
    ///
    /// A deterministic page containing one allowed and one hidden hit.
    ///
    /// This fixture does not fail discovery.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find(&self, request: &FindRequest) -> Result<FindResult, QueryError> {
        Ok(FindResult {
            scope: request.scope.clone(),
            strategy: request.strategy.clone(),
            page: Page {
                items: vec![hit(ALLOWED), hit(RESTRICTED)],
                next: None,
            },
            information_cost: None,
        })
    }
}

/// Fails if a view request reaches the provider without a render grant.
struct InspectFixture;

impl InspectService for InspectFixture {
    /// Panics if authorization fails to stop a restricted view before dispatch.
    ///
    /// - `_request`: View request that should be denied first.
    ///
    /// Never returns a successful inspection.
    ///
    /// Always panics because the provider must not be reached.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(&self, _request: &InspectRequest) -> Result<Inspection, QueryError> {
        panic!("restricted view reached provider")
    }
}

/// Builds a compact navigation entry for one target entity.
///
/// - `entity`: Entity represented by the entry.
///
/// A snapshot-bound compact navigation entry.
/// # Returns
/// The operation result.
fn entry(entity: EntityId) -> NavigationEntry {
    NavigationEntry {
        target: TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
            entity,
            snapshot: SNAPSHOT,
        }),
        entity_type: None,
        label: None,
        description: None,
        metadata: None,
    }
}

/// Builds a discovery hit for one target entity.
///
/// - `entity`: Entity represented by the hit.
///
/// A snapshot-bound discovery hit.
/// # Returns
/// The operation result.
fn hit(entity: EntityId) -> FindHit {
    FindHit {
        target: TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
            entity,
            snapshot: SNAPSHOT,
        }),
        entity_type: Some(ObjectType::new("test.item").expect("type")),
        label: Some("safe label".to_owned()),
        preview: Some("safe preview".to_owned()),
        score: None,
        anchor: None,
        provenance: None,
    }
}

/// Constructs a relation request rooted at the authorized entity.
///
/// A deterministic outgoing relation request.
/// # Returns
/// The operation result.
fn follow_request() -> FollowRequest {
    FollowRequest {
        subject: scriberig_query::EntityInSnapshot {
            entity: ROOT,
            snapshot: SNAPSHOT,
        },
        relation: scriberig_query::RelationRef {
            source: ROOT,
            relation_type: "depends_on".to_owned(),
            target: None,
        },
        direction: FollowDirection::Outgoing,
        page: PageRequest::default(),
        detail: scriberig_query::DetailLevel::Compact,
    }
}

/// Allows the relation root and one child while filtering the restricted child.
#[test]
fn follow_filters_restricted_neighbors() {
    let guarded = GuardedQueryService::new(TraversalFixture);
    let context = context([
        grant(
            CapabilityScope::Relation {
                entity: ROOT,
                relation: "depends_on".to_owned(),
            },
            CapabilityAction::Query,
        ),
        grant(CapabilityScope::Entity(ALLOWED), CapabilityAction::Query),
    ]);

    let result = guarded
        .follow(&context, &follow_request())
        .expect("authorized relation");
    assert_eq!(result.page.items.len(), 1);
    assert_eq!(result.page.items[0].target, entry(ALLOWED).target);
}

/// Refuses project-wide discovery when the caller only has entity authority.
#[test]
fn global_find_requires_project_scope() {
    let guarded = GuardedQueryService::new(FindFixture);
    let context = context([grant(
        CapabilityScope::Entity(ALLOWED),
        CapabilityAction::Query,
    )]);
    let request = FindRequest {
        scope: QueryScope::Snapshot(SNAPSHOT),
        query: "secret".to_owned(),
        strategy: FindStrategy::ExactText,
        page: PageRequest::default(),
        detail: scriberig_query::DetailLevel::Compact,
    };

    let error = guarded.find(&context, &request).expect_err("must deny");
    assert!(matches!(error, ApplicationError::PermissionDenied { .. }));
}

/// Requires a render grant in addition to subject inspection authority.
#[test]
fn derived_view_requires_render_scope() {
    let guarded = GuardedQueryService::new(InspectFixture);
    let context = context([grant(
        CapabilityScope::Entity(ALLOWED),
        CapabilityAction::Inspect,
    )]);
    let request = InspectRequest {
        target: TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
            entity: ALLOWED,
            snapshot: SNAPSHOT,
        }),
        detail: scriberig_query::DetailLevel::Compact,
        view: Some(ProjectionRef("summary".to_owned())),
    };

    let error = guarded
        .inspect_target(&context, &request)
        .expect_err("must deny view without render authority");
    assert!(matches!(error, ApplicationError::PermissionDenied { .. }));
}

/// Confirms the reusable provider remains callable without application policy.
#[test]
fn low_level_provider_remains_policy_free() {
    let request = follow_request();
    let result = TraversalFixture
        .follow(&request)
        .expect("provider is reusable");
    assert_eq!(result.page.items.len(), 2);
}

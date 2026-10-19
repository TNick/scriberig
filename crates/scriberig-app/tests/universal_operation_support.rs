//! Provides shared fixtures for universal application integration tests.

#![allow(dead_code)]
#![allow(clippy::duplicate_mod)]

use scriberig_app::*;
use scriberig_core::*;
use scriberig_query::{
    FindRequest, FindResult, FindService, FollowRequest, FollowResult, InspectRequest,
    InspectService, Inspection, ListRequest, ListResult, QueryError, QueryResult, QuerySpec,
    StructuredQueryService, TraverseService,
};

mod operation_fixture;
pub use operation_fixture::OperationFixture;

/// Provides explicit unsupported errors for read-dispatch assertions.
pub struct ReadFixture;

/// Provides one successful inspection and unsupported traversal operations.
pub struct InspectingReadFixture;

impl InspectService for InspectingReadFixture {
    /// Returns one generic inspection for capability-enrichment assertions.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(&self, request: &InspectRequest) -> Result<Inspection, QueryError> {
        Ok(Inspection {
            subject: request.target,
            snapshot: Some(SnapshotId::new([2; 32])),
            revision: None,
            entity_type: Some(ObjectType::new("test.entity").expect("type")),
            label: None,
            description: None,
            properties: Vec::new(),
            counts: Vec::new(),
            relations: Vec::new(),
            collections: Vec::new(),
            views: Vec::new(),
            operations: Vec::new(),
            provenance: Vec::new(),
            information_cost: None,
            detail: request.detail,
        })
    }
}

impl TraverseService for InspectingReadFixture {
    /// Returns an explicit unsupported list capability.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list(&self, request: &ListRequest) -> Result<ListResult, QueryError> {
        ReadFixture.list(request)
    }

    /// Returns an explicit unsupported follow capability.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn follow(&self, request: &FollowRequest) -> Result<FollowResult, QueryError> {
        ReadFixture.follow(request)
    }
}

impl FindService for InspectingReadFixture {
    /// Returns an explicit unsupported find capability.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find(&self, request: &FindRequest) -> Result<FindResult, QueryError> {
        ReadFixture.find(request)
    }
}

impl StructuredQueryService for InspectingReadFixture {
    /// Returns an explicit unsupported query capability.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn query(&self, request: &QuerySpec) -> Result<QueryResult, QueryError> {
        ReadFixture.query(request)
    }
}

/// Advertises one domain capability for inspection enrichment.
pub struct InspectionCapabilityProvider;

impl CapabilityProvider for InspectionCapabilityProvider {
    /// Returns relation, view, and operation descriptors for test entities.
    ///
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<CapabilityDescriptor> {
        let entity_type = ObjectType::new("test.entity").expect("type");
        vec![
            CapabilityDescriptor {
                id: CapabilityId::new("relation.calls").expect("capability"),
                kind: CapabilityKind::Relation,
                label: "Calls".to_owned(),
                description: "Outgoing calls".to_owned(),
                entity_types: vec![entity_type.clone()],
                available: true,
                derived: false,
                estimated_items: None,
            },
            CapabilityDescriptor {
                id: CapabilityId::new("view.summary").expect("capability"),
                kind: CapabilityKind::View,
                label: "Summary".to_owned(),
                description: "Summary projection".to_owned(),
                entity_types: vec![entity_type.clone()],
                available: true,
                derived: false,
                estimated_items: None,
            },
            CapabilityDescriptor {
                id: CapabilityId::new("operation.rename").expect("capability"),
                kind: CapabilityKind::DomainOperation,
                label: "Rename".to_owned(),
                description: "Rename the entity".to_owned(),
                entity_types: vec![entity_type],
                available: true,
                derived: false,
                estimated_items: None,
            },
        ]
    }

    /// Returns one exact-base change handle for the rename operation.
    ///
    /// - `request`: Authorized semantic subject and typed arguments.
    ///
    /// A candidate change handle bound to the selected entity and snapshot.
    ///
    /// Returns unsupported-capability for another operation identifier.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        request: &DomainOperationRequest,
    ) -> Result<CapabilityExecution, ApplicationError> {
        if request.capability.as_str() != "operation.rename" {
            return Err(ApplicationError::UnsupportedCapability {
                operation: "domain operation".to_owned(),
                capability: request.capability.to_string(),
            });
        }
        Ok(CapabilityExecution {
            change: format!(
                "candidate:{}:{}",
                request.subject.entity, request.context.base_snapshot
            ),
        })
    }
}

impl InspectService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// - `self`: Fixture provider.
    /// - `_request`: Inspection request.
    ///
    /// Never returns an inspection.
    ///
    /// Always returns the fixture capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(&self, _request: &InspectRequest) -> Result<Inspection, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Inspect,
            capability: "fixture".to_owned(),
        })
    }
}

impl TraverseService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// - `self`: Fixture provider.
    /// - `_request`: Collection request.
    ///
    /// Never returns a collection page.
    ///
    /// Always returns the fixture capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list(&self, _request: &ListRequest) -> Result<ListResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::List,
            capability: "fixture".to_owned(),
        })
    }

    /// Returns an explicit provider error.
    ///
    /// - `self`: Fixture provider.
    /// - `_request`: Relation request.
    ///
    /// Never returns a traversal page.
    ///
    /// Always returns the fixture capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn follow(&self, _request: &FollowRequest) -> Result<FollowResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Follow,
            capability: "fixture".to_owned(),
        })
    }
}

impl FindService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// - `self`: Fixture provider.
    /// - `_request`: Discovery request.
    ///
    /// Never returns discovery results.
    ///
    /// Always returns the fixture capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find(&self, _request: &FindRequest) -> Result<FindResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Find,
            capability: "fixture".to_owned(),
        })
    }
}

impl StructuredQueryService for ReadFixture {
    /// Returns an explicit provider error.
    ///
    /// - `self`: Fixture provider.
    /// - `_specification`: Structural query request.
    ///
    /// Never returns query rows.
    ///
    /// Always returns the fixture capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn query(&self, _specification: &QuerySpec) -> Result<QueryResult, QueryError> {
        Err(QueryError::UnsupportedCapability {
            operation: scriberig_query::ReadOperation::Query,
            capability: "fixture".to_owned(),
        })
    }
}

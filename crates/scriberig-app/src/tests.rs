//! Tests the application service facade.

use super::{ApplicationBackend, ApplicationService};
use crate::{
    AgentConsoleBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, CapabilityContext, CapabilityId, DetailLevel, InspectRequest, QueryService,
    SnapshotResolution,
};
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_query::{EntityAtRevision, Inspection, TargetRef};

/// Provides one deterministic backend inspection for service tests.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Executes the fixture inspection request.
    ///
    /// - `self`: Fixture backend.
    /// - `_context`: Unused test context.
    /// - `request`: Request expected to be an inspection.
    ///
    /// One deterministic inspection result.
    ///
    /// Returns an internal error for another request kind.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let ApplicationRequest::Inspect(_) = request else {
            return Err(ApplicationError::UnsupportedOperation {
                operation: "fixture".to_owned(),
            });
        };
        let entity = EntityId::from_bytes([1; 16]);
        let revision = RevisionId::new([2; 32]);
        let snapshot = SnapshotId::new([3; 32]);
        Ok(ApplicationResult::Inspect(Inspection {
            subject: TargetRef::Revision(EntityAtRevision { entity, revision }),
            snapshot: Some(snapshot),
            revision: Some(revision),
            entity_type: Some(ObjectType::new("test.entity").expect("valid type")),
            label: Some("fixture".to_owned()),
            description: Some("test inspection".to_owned()),
            properties: Vec::new(),
            counts: Vec::new(),
            relations: Vec::new(),
            collections: Vec::new(),
            views: Vec::new(),
            operations: Vec::new(),
            provenance: Vec::new(),
            information_cost: None,
            detail: scriberig_query::DetailLevel::Identity,
        }))
    }

    /// Resolves the fixture's exact current snapshot.
    ///
    /// - `self`: Fixture backend.
    /// - `_context`: Unused test context.
    ///
    /// The deterministic current snapshot.
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<SnapshotId, ApplicationError> {
        Ok(SnapshotId::new([3; 32]))
    }
}

/// Verifies the typed inspection facade uses the universal execution path.
#[test]
fn inspection_delegates_through_application_backend() {
    let service = ApplicationService::new(Fixture);
    let request = InspectRequest {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotResolution::At(SnapshotId::new([3; 32])),
        detail: DetailLevel::Compact,
    };
    let result = service
        .inspect(&ApplicationContext::empty(), &request)
        .expect("inspection");
    assert_eq!(result.entity, request.entity);
    assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
    assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
}

/// Verifies omitted current state is resolved before universal execution.
#[test]
fn current_inspection_is_resolved_to_one_exact_snapshot() {
    let service = ApplicationService::new(Fixture);
    let request = InspectRequest {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotResolution::Current,
        detail: DetailLevel::Compact,
    };
    let result = service
        .inspect(&ApplicationContext::empty(), &request)
        .expect("current inspection");
    assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
}

/// Verifies a changed upstream catalog is reflected by a newly composed service.
#[test]
fn model_library_projection_tracks_recomposed_upstream_catalog() {
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_model_library").expect("valid model-library capability")
        ]),
        ..ApplicationContext::default()
    };
    let original = rpi_ai::Model::new(
        "wire-a",
        "Model A",
        rpi_ai::Api::AnthropicMessages,
        "provider-a",
        "https://example.test",
    );
    let first_service =
        ApplicationService::new(Fixture).with_upstream_models(vec![original.clone()]);

    let mut updated = original;
    updated.name = "Updated Model A".to_owned();
    updated.context_window = 128_000;
    let added = rpi_ai::Model::new(
        "wire-b",
        "Model B",
        rpi_ai::Api::AnthropicMessages,
        "provider-b",
        "https://example.test",
    );
    let refreshed_service =
        ApplicationService::new(Fixture).with_upstream_models(vec![updated, added]);

    let first = AgentConsoleBackend::agent_model_library(&first_service, &context)
        .expect("initial upstream projection");
    let refreshed = AgentConsoleBackend::agent_model_library(&refreshed_service, &context)
        .expect("refreshed upstream projection");

    assert_eq!(first.models.len(), 1);
    assert_eq!(refreshed.models.len(), 2);
    assert!(refreshed.models.iter().any(|model| {
        model.metadata_snapshot.model_id == "wire-a"
            && model.metadata_snapshot.display_name == "Updated Model A"
            && model.metadata_snapshot.context_window == 128_000
    }));
    assert!(
        refreshed
            .models
            .iter()
            .any(|model| model.metadata_snapshot.provider_id == "provider-b"
                && model.metadata_snapshot.model_id == "wire-b")
    );
}

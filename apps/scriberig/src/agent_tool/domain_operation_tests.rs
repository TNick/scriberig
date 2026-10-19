//! Verifies semantic domain operations reach the shared application boundary.

use std::sync::{Arc, Mutex};

use scriberig_agent_core::Tool;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityExecution, CapabilityId, CapabilityScope, DomainOperationRequest, OperationKind,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::QueryValue;

use crate::agent_tool::LocalAgentTool;

/// Records typed domain requests at the application boundary.
struct FakeDomainBackend {
    /// Captured application request for assertion.
    request: Arc<Mutex<Option<DomainOperationRequest>>>,
}

impl ApplicationBackend for FakeDomainBackend {
    /// Captures one semantic request and returns its candidate change handle.
    ///
    /// - `self`: Fake application backend.
    /// - `_context`: Trusted application context.
    /// - `request`: Shared application request.
    ///
    /// Candidate change result for a domain operation.
    ///
    /// Returns invalid-request for unrelated application operations.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let ApplicationRequest::DomainOperation(request) = request else {
            return Err(ApplicationError::InvalidRequest {
                operation: "fixture".to_owned(),
                message: "expected domain operation".to_owned(),
            });
        };
        *self.request.lock().expect("request lock") = Some(request.clone());
        Ok(ApplicationResult::DomainOperation(CapabilityExecution {
            change: "candidate:typed-rename".to_owned(),
        }))
    }
}

/// Verifies provider arguments become an exact scoped shared request.
#[test]
fn named_domain_tool_builds_exact_semantic_application_request() {
    let entity = EntityId::from_bytes([91; 16]);
    let snapshot = SnapshotId::new([92; 32]);
    let captured = Arc::new(Mutex::new(None));
    let service = scriberig_app::ApplicationService::new(FakeDomainBackend {
        request: Arc::clone(&captured),
    });
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext {
            granted: vec![CapabilityId::new("domain_operation").expect("grant")],
            grants: vec![Capability {
                scope: CapabilityScope::Entity(entity),
                actions: [CapabilityAction::Custom("code.rename".to_owned())]
                    .into_iter()
                    .collect(),
                constraints: CapabilityConstraints {
                    snapshot: Some(snapshot),
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            }],
        },
        ..ApplicationContext::empty()
    };
    let tool = LocalAgentTool::new(&service);

    let result = tool
        .invoke_named(
            &context,
            "domain_operation",
            &serde_json::json!({
                "capability": "code.rename",
                "entity": entity.to_string(),
                "snapshot": snapshot.to_string(),
                "branch": "feature",
                "arguments": {"name": "renamed", "position": 3, "exported": true}
            }),
        )
        .expect("semantic tool request");

    assert_eq!(result.object_type.as_deref(), Some("semantic_candidate"));
    assert!(
        result
            .handles
            .contains(&"candidate_change:candidate:typed-rename".to_owned())
    );
    let request = captured
        .lock()
        .expect("request lock")
        .clone()
        .expect("application request captured");
    assert_eq!(request.capability.as_str(), "code.rename");
    assert_eq!(request.subject.entity, entity);
    assert_eq!(request.subject.snapshot, snapshot);
    assert_eq!(request.context.base_snapshot, snapshot);
    assert_eq!(request.context.branch.unwrap().as_str(), "feature");
    assert!(
        request
            .arguments
            .contains(&("name".to_owned(), QueryValue::Text("renamed".to_owned())))
    );
    assert!(
        request
            .arguments
            .contains(&("position".to_owned(), QueryValue::Integer(3)))
    );
    assert!(
        request
            .arguments
            .contains(&("exported".to_owned(), QueryValue::Boolean(true)))
    );
}

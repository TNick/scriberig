//! Verifies isolated plugin host capability dispatch through application policy.

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityExecution, CapabilityId, CapabilityScope, DomainOperationRequest, OperationKind,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::{QueryValue, Selection};

use super::start_fixture_worker_with_policy;
use scriberig_agent_runtime::plugin_worker::PluginSemanticDispatcher;

/// Captures the public semantic request dispatched by a plugin worker.
struct RecordingOperationBackend {
    /// Request observed at the application boundary.
    request: Arc<std::sync::Mutex<Option<DomainOperationRequest>>>,
}

impl ApplicationBackend for RecordingOperationBackend {
    /// Records the exact domain operation and returns a candidate handle.
    ///
    /// # Parameters
    /// - `self`: Recording backend.
    /// - `_context`: Trusted application context.
    /// - `request`: Application operation requested by the plugin dispatcher.
    ///
    /// # Returns
    /// A candidate change result for a domain operation.
    ///
    /// # Errors
    /// Returns an error when the dispatcher sends another application request.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let ApplicationRequest::DomainOperation(operation) = request else {
            return Err(ApplicationError::InvalidRequest {
                operation: "fixture".to_owned(),
                message: "expected domain operation".to_owned(),
            });
        };
        *self.request.lock().expect("recording lock") = Some(operation.clone());
        Ok(ApplicationResult::DomainOperation(CapabilityExecution {
            change: "candidate:plugin-domain-operation".to_owned(),
        }))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "integration scenario verifies the complete semantic operation boundary"
)]
/// Verifies plugin tools reach public semantic operations through host policy.
#[test]
fn plugin_tool_uses_host_capability_for_scoped_semantic_operation() {
    let launcher = scriberig_agent_runtime::plugin_worker::sandboxed_launcher()
        .expect("verified sandbox launcher");
    let plugin = super::provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical root"),
    ];
    let snapshot = SnapshotId::new([31; 32]);
    let entity = EntityId::from_bytes([32; 16]);
    let recorded = Arc::new(std::sync::Mutex::new(None));
    let service = scriberig_app::ApplicationService::new(RecordingOperationBackend {
        request: Arc::clone(&recorded),
    });
    let operation_capability = CapabilityId::new("fixture.rename").expect("operation id");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext {
            granted: vec![CapabilityId::new("domain_operation").expect("dispatcher grant")],
            grants: vec![Capability {
                scope: CapabilityScope::Entity(entity),
                actions: [CapabilityAction::Custom(operation_capability.to_string())]
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
    let component_mailbox = rpi_extensions::ExtensionStatusMailbox::new();
    let dispatcher: Arc<dyn PluginSemanticDispatcher> = Arc::new(
        scriberig_agent_runtime::plugin_worker::ApplicationDomainDispatcher::new(
            Arc::new(service),
            context,
            None,
        )
        .with_extension_components(component_mailbox.clone()),
    );
    let worker = start_fixture_worker_with_policy(
        &plugin,
        &protected_roots,
        launcher,
        std::collections::BTreeSet::from(["scriberig_fixture_tool".to_owned()]),
        std::collections::BTreeSet::from([
            "domain_operation".to_owned(),
            "ui_component".to_owned(),
        ]),
        Arc::clone(&dispatcher),
    );
    let subject = Selection {
        snapshot,
        entity,
        revision: None,
        relation_path: Vec::new(),
        projection_anchor: None,
    };
    let result = worker
        .call_tool(
            "scriberig_fixture_tool",
            serde_json::json!({
                "hostCapability": {
                    "name": "domain_operation",
                    "arguments": {
                        "capability": "fixture.rename",
                        "subject": subject,
                        "arguments": [["name", QueryValue::Text("renamed".to_owned())]]
                    }
                },
                "hostComponent": {
                    "op":"register",
                    "id":"worker-status",
                    "type":"text",
                    "slot":"footer.after",
                    "width":60,
                    "maxHeight":1,
                    "border":false
                }
            }),
        )
        .expect("explicitly allowlisted host semantic operation");
    assert!(
        result
            .to_string()
            .contains("candidate:plugin-domain-operation"),
        "plugin tool did not receive the public operation result: {result}"
    );
    let operation = recorded
        .lock()
        .expect("recording lock")
        .clone()
        .expect("public semantic operation was dispatched");
    assert_eq!(operation.capability, operation_capability);
    assert_eq!(operation.subject.snapshot, snapshot);
    assert_eq!(operation.subject.entity, entity);
    assert_eq!(
        operation.arguments,
        vec![("name".to_owned(), QueryValue::Text("renamed".to_owned()))]
    );
    assert_eq!(component_mailbox.panels().len(), 1);
    drop(worker);
    drop(dispatcher);
    assert!(component_mailbox.panels().is_empty());
}

/// Verifies worker component calls publish to and clean up the shared UI mailbox.
#[test]
fn plugin_ui_component_host_call_is_scoped_and_removed_with_dispatcher() {
    let service = Arc::new(scriberig_app::ApplicationService::new(
        RecordingOperationBackend {
            request: Arc::new(std::sync::Mutex::new(None)),
        },
    ));
    let mailbox = rpi_extensions::ExtensionStatusMailbox::new();
    let dispatcher = scriberig_agent_runtime::plugin_worker::ApplicationDomainDispatcher::new(
        service,
        ApplicationContext::empty(),
        None,
    )
    .with_extension_components(mailbox.clone());

    let result = dispatcher
        .dispatch(
            "ui_component",
            &serde_json::json!({
                "owner": 12,
                "arguments": {
                    "op": "register",
                    "id": "worker-status",
                    "type": "text",
                    "slot": "footer.after",
                    "width": 60,
                    "maxHeight": 1,
                    "border": false
                }
            }),
        )
        .expect("host accepts isolated component registration");

    assert_eq!(result["ok"], true);
    assert_eq!(mailbox.panels().len(), 1);
    drop(dispatcher);
    assert!(mailbox.panels().is_empty());
}

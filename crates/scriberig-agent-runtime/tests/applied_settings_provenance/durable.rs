//! Persists one upstream applied-settings report and reads it back through
//! the authorized model-request detail operation.

use std::sync::Arc;

use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, ApplicationBackend, ApplicationContext,
    ApplicationError, ApplicationRequest, ApplicationResult, ApplicationService, CapabilityContext,
    CapabilityId, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session_sqlite::SqliteSessionStore;

use super::fixture::{CREDENTIAL_SENTINEL, ReportMode, capture, fixture_model};

/// Rejects universal semantic requests because this fixture owns no backend state.
struct NoUniversalBackend;

impl ApplicationBackend for NoUniversalBackend {
    /// Rejects universal requests because only application session state is used.
    ///
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns unsupported-operation for every request.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture universal request".to_owned(),
        })
    }
}

/// Durable fixture holding one completed root branch and its SQLite file.
struct DurableCapture {
    /// Temporary directory owning the SQLite file and journals.
    _directory: tempfile::TempDir,
    /// Path of the durable session store.
    path: std::path::PathBuf,
    /// Report captured at the provider boundary.
    report: serde_json::Value,
    /// Captured request projection carrying that report.
    captured: Vec<scriberig_session::ModelRequestProjection>,
    /// Session owning the completed root branch.
    session: EntityId,
    /// Completed root branch.
    root_branch: EntityId,
}

impl DurableCapture {
    /// Captures one attested report and persists it through the shared operation.
    ///
    /// # Returns
    /// The durable fixture identities for the completed root branch.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary session directory");
        let path = directory.path().join("scriberig-applied-settings.sqlite");
        let captured = capture(ReportMode::Attested);
        let report = captured[0]
            .applied_settings
            .as_ref()
            .expect("captured upstream report")
            .clone();
        let catalog = rpi_model_runtime::ModelCatalog::new();
        catalog.reconcile_models(vec![fixture_model(
            "fixture-applied-settings",
            "applied-settings-model",
        )]);
        let session = EntityId::from_bytes([82; 16]);
        let store: Arc<dyn scriberig_session::SessionStore> =
            Arc::new(SqliteSessionStore::open(&path).expect("open durable session store"));
        let writer = Self::service(&catalog, &store);
        let handle = writer
            .start_local_agent(&AgentCommandRequest {
                session,
                branch: None,
                anchor: None,
                prompt: "review the bounded request".to_owned(),
                selection: None,
            })
            .expect("start local agent");
        writer
            .complete_local_agent_with_evidence(
                &handle,
                "safe completed answer",
                Vec::new(),
                Vec::new(),
                captured.clone(),
            )
            .expect("persist the captured report");
        Self {
            _directory: directory,
            path,
            report,
            captured,
            session,
            root_branch: handle.root_branch,
        }
    }

    /// Builds an application service over a fresh SQLite handle.
    ///
    /// - `catalog`: Upstream model catalog supplied to the service.
    /// - `store`: Durable session store, when the writer needs the same handle.
    ///
    /// # Returns
    /// The application service for reads or writes.
    fn service(
        catalog: &rpi_model_runtime::ModelCatalog,
        store: &Arc<dyn scriberig_session::SessionStore>,
    ) -> ApplicationService<NoUniversalBackend> {
        ApplicationService::new(NoUniversalBackend)
            .with_session_store(Arc::clone(store))
            .with_model_catalog(catalog.clone())
    }

    /// Reopens the durable session store and returns a reader for it.
    ///
    /// # Returns
    /// A service reading only the durable graph.
    fn reader(&self) -> ApplicationService<NoUniversalBackend> {
        let catalog = rpi_model_runtime::ModelCatalog::new();
        catalog.reconcile_models(vec![fixture_model(
            "fixture-applied-settings",
            "applied-settings-model",
        )]);
        let store: Arc<dyn scriberig_session::SessionStore> =
            Arc::new(SqliteSessionStore::open(&self.path).expect("reopen durable session store"));
        Self::service(&catalog, &store)
    }

    /// Reads the durable model-invocation identity for the root branch.
    ///
    /// # Returns
    /// The stable `ScribeRig` model-invocation identity.
    fn invocation_id(&self) -> EntityId {
        let reader = self.reader();
        let durable = SessionBackend::read_session(
            &reader,
            &context(true),
            &SessionRequest {
                session: self.session,
            },
        )
        .expect("reload the durable session graph");
        durable.branches[0].model_invocations[0].id
    }
}

/// Builds a session-read caller context.
///
/// - `read`: Whether the context grants session-tree reads.
///
/// # Returns
/// The application request context.
///
/// # Panics
/// Panics when a static capability identifier is invalid.
fn context(read: bool) -> ApplicationContext {
    let capabilities = if read {
        CapabilityContext::new([CapabilityId::new("session_branch_tree").expect("session read")])
    } else {
        CapabilityContext::default()
    };
    ApplicationContext {
        capabilities,
        ..ApplicationContext::empty()
    }
}

/// Proves one upstream report survives durable capture and the detail read.
#[test]
fn applied_settings_survive_durable_capture_and_authorized_detail() {
    let fixture = DurableCapture::new();
    let reader = fixture.reader();
    let authorized = context(true);
    let detail = AgentConsoleBackend::model_request_detail(
        &reader,
        &authorized,
        fixture.session,
        fixture.root_branch,
        fixture.invocation_id(),
    )
    .expect("authorized historical request detail");
    assert_eq!(
        detail.applied_settings.as_ref(),
        Some(&fixture.report),
        "the persisted report must equal the report captured at the provider boundary"
    );
    assert_eq!(
        detail.applied_settings_fingerprint, fixture.captured[0].applied_settings_fingerprint,
        "the versioned report digest must survive persistence"
    );
    assert_eq!(
        detail.request_fingerprint,
        fixture.captured[0].request_fingerprint
    );
    let persisted = serde_json::to_value(&detail).expect("serialize the detail response");
    assert_eq!(
        persisted["applied_settings"]["complete"],
        serde_json::json!(false),
        "an upstream partial report must stay explicitly incomplete"
    );
    assert!(
        persisted["applied_settings"]["omittedCategories"]
            .as_array()
            .is_some_and(|categories| !categories.is_empty()),
        "the persisted report must keep its explicit omissions"
    );
    assert_ne!(
        persisted["applied_settings"], persisted["model_execution_configuration"],
        "catalog-derived execution metadata is a separate observation"
    );
    assert!(
        !serde_json::to_string(&detail)
            .expect("serialize the detail")
            .contains(CREDENTIAL_SENTINEL),
        "the authorized detail read must not carry catalog credentials"
    );
}

/// Keeps the report durable while restricting it to authorized readers.
#[test]
fn applied_settings_detail_requires_the_session_read_grant() {
    let fixture = DurableCapture::new();
    let reader = fixture.reader();
    let invocation = fixture.invocation_id();
    let denied = AgentConsoleBackend::model_request_detail(
        &reader,
        &context(false),
        fixture.session,
        fixture.root_branch,
        invocation,
    );
    assert!(
        matches!(denied, Err(ApplicationError::PermissionDenied { .. })),
        "an unauthorized caller must not read the applied-settings report: {denied:?}"
    );
    let stored = std::fs::read(&fixture.path).expect("read the durable SQLite file");
    let stored = String::from_utf8_lossy(&stored);
    assert!(
        stored.contains("rpi-applied-request-settings-v1"),
        "the report and its contract version must be durable"
    );
    assert!(
        !stored.contains(CREDENTIAL_SENTINEL),
        "the durable payload must not carry catalog credentials"
    );
}

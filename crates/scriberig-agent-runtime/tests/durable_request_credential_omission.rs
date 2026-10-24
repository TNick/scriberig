//! Verifies provider credentials stay out of durable model-request history.

use std::collections::BTreeMap;
use std::sync::Arc;

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation_blocking,
};
use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, ApplicationBackend, ApplicationContext,
    ApplicationError, ApplicationRequest, ApplicationResult, ApplicationService, CapabilityContext,
    CapabilityId, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::ModelRequestProjection;
use scriberig_session_sqlite::SqliteSessionStore;

/// Header and endpoint credential text that must never become a durable record.
const CREDENTIAL_SENTINEL: &str = "durable-request-credential-sentinel-8f31";

/// Rejects universal semantic requests because this fixture owns no backend state.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal requests because only application session state is used.
    ///
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
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

/// Builds the authorized session-read and model-library caller context.
///
/// The context granting session-tree reads and safe model-library reads.
/// # Returns
/// The operation result.
/// # Panics
/// Panics when a static capability identifier is invalid.
#[must_use]
fn authorized_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session read capability"),
            CapabilityId::new("agent_model_library").expect("model library capability"),
        ]),
        ..ApplicationContext::empty()
    }
}

/// Runs one real upstream branch invocation whose model carries credentials.
///
/// - `provider`: Deterministic upstream provider for the branch invocation.
/// - `model`: Exact upstream model carrying the credential sentinel.
///
/// The request projections captured before provider serialization.
/// # Returns
/// The operation result.
/// # Panics
/// Panics when the deterministic upstream invocation fails.
fn capture_requests(
    provider: Arc<dyn rpi_ai::Provider>,
    model: &rpi_ai::Model,
) -> Vec<ModelRequestProjection> {
    let compiled_context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "session",
            "branch",
            "credential omission proof",
            "review the bounded request",
        ));
    let invocation = UpstreamBranchInvocation {
        branch: EntityId::from_bytes([61; 16]),
        provider,
        model: model.clone(),
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: Some("host-provider-profile".to_owned()),
        thinking_level: rpi_ai::ThinkingLevel::Off,
        context: compiled_context,
        tools: Vec::new(),
    };
    run_upstream_branch_invocation_blocking(invocation, "review the bounded request")
        .expect("upstream branch invocation")
        .model_requests
}

/// Reports whether one serialized value contains the credential sentinel.
///
/// - `value`: Session graph, capture, or detail value to inspect.
/// - `subject`: Value name used in the panic message.
///
/// Whether the serialized form contains the credential sentinel.
/// # Returns
/// The operation result.
/// # Panics
/// Panics when the value cannot be serialized.
fn serialized_contains_credential<T: serde::Serialize>(value: &T, subject: &str) -> bool {
    let serialized = serde_json::to_string(value)
        .unwrap_or_else(|error| panic!("could not serialize {subject}: {error}"));
    serialized.contains(CREDENTIAL_SENTINEL)
}

/// Proves a credential sentinel is omitted from capture, SQLite reload, and detail.
#[test]
#[allow(clippy::too_many_lines)] // Keeps capture, persistence, reload, and detail in one scenario.
fn credential_sentinels_never_reach_durable_model_request_detail() {
    // The temporary directory owns the SQLite file and any sidecar journals, so
    // cleanup still runs when an assertion below panics.
    let directory = tempfile::tempdir().expect("temporary session directory");
    let path = directory
        .path()
        .join("scriberig-credential-omission.sqlite");
    let provider = FauxProvider::new(FauxScript::new().with_text("safe completed answer"));
    let mut model = provider.default_model().clone();
    model.base_url = format!("https://example.invalid/v1?token={CREDENTIAL_SENTINEL}");
    model.headers = Some(BTreeMap::from([(
        "authorization".to_owned(),
        format!("Bearer {CREDENTIAL_SENTINEL}"),
    )]));
    let catalog = rpi_model_runtime::ModelCatalog::new();
    catalog.reconcile_models(vec![model.clone()]);

    let captured = capture_requests(provider.clone(), &model);
    assert_eq!(captured.len(), 1);
    assert!(
        !serialized_contains_credential(&captured, "captured request projection"),
        "request capture leaked the provider credential sentinel"
    );

    let session = EntityId::from_bytes([62; 16]);
    let store: Arc<dyn scriberig_session::SessionStore> =
        Arc::new(SqliteSessionStore::open(&path).expect("open durable session store"));
    let writer = ApplicationService::new(Fixture)
        .with_session_store(Arc::clone(&store))
        .with_model_catalog(catalog.clone());
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
        .expect("persist the captured request");

    // A second service on a fresh SQLite handle reads only the durable graph.
    let context = authorized_context();
    let reader = ApplicationService::new(Fixture)
        .with_session_store(Arc::new(
            SqliteSessionStore::open(&path).expect("reopen durable session store"),
        ))
        .with_model_catalog(catalog.clone());
    let durable = SessionBackend::read_session(&reader, &context, &SessionRequest { session })
        .expect("reload the durable session graph");
    assert!(
        !serialized_contains_credential(&durable, "reloaded durable session"),
        "the SQLite reload leaked the provider credential sentinel"
    );
    let invocation_id = durable.branches[0].model_invocations[0].id;

    let library = AgentConsoleBackend::agent_model_library(&reader, &context)
        .expect("authorized upstream model library");
    assert_eq!(
        library
            .iter()
            .map(|row| row.model_id.as_str())
            .collect::<Vec<_>>(),
        vec![model.id.as_str()]
    );

    let detail = AgentConsoleBackend::model_request_detail(
        &reader,
        &context,
        session,
        handle.root_branch,
        invocation_id,
    )
    .expect("authorized historical request detail");
    assert_eq!(detail, captured[0]);
    assert_eq!(detail.provider_id.as_deref(), Some(model.provider.as_str()));
    assert_eq!(detail.model, model.id);
    assert!(detail.request_fingerprint.is_some());
    assert!(
        !serialized_contains_credential(&detail, "authorized request detail"),
        "the authorized request detail leaked the provider credential sentinel"
    );
    assert!(
        !serialized_contains_credential(
            &durable.branches[0].model_invocations,
            "durable invocation"
        ),
        "the durable model invocation leaked the provider credential sentinel"
    );
    assert!(
        detail
            .messages
            .iter()
            .any(|message| message.content.contains("review the bounded request")),
        "the captured safe request content must survive persistence"
    );
    let safe_configuration = detail
        .model_execution_configuration
        .as_ref()
        .expect("upstream safe model execution configuration");
    let exclusions = safe_configuration["excludedExecutionFields"]
        .as_array()
        .expect("upstream execution-field omission report");
    for excluded in ["endpoint", "headers"] {
        assert!(
            exclusions
                .iter()
                .any(|value| value.as_str() == Some(excluded)),
            "upstream omission metadata must name the excluded {excluded} field"
        );
    }

    // Removing every live catalog row must not change the historical detail.
    catalog.reconcile_models(Vec::new());
    let refreshed =
        AgentConsoleBackend::agent_model_library(&reader, &context).expect("refreshed model rows");
    assert_eq!(refreshed.len(), 0);
    let after_mutation = AgentConsoleBackend::model_request_detail(
        &reader,
        &context,
        session,
        handle.root_branch,
        invocation_id,
    )
    .expect("historical request detail after catalog mutation");
    assert_eq!(after_mutation, detail);
    assert_eq!(
        after_mutation.provider_id.as_deref(),
        Some(model.provider.as_str())
    );

    drop(reader);
    drop(writer);
    drop(store);
    let stored_bytes = std::fs::read(&path).expect("read the durable SQLite file");
    assert!(
        !String::from_utf8_lossy(&stored_bytes).contains(CREDENTIAL_SENTINEL),
        "the durable SQLite payload leaked the provider credential sentinel"
    );
}

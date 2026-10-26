//! Verifies an isolated provider reaches the network only through host RPC.

use super::support::{provider_plugin_artifact, start_fixture_worker_with_environment};
use rpi_ai::{Provider, ThinkingLevel};
use scriberig_agent_runtime::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};
use scriberig_agent_runtime::plugin_worker::{PluginSemanticDispatcher, sandboxed_launcher};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchInvocation, run_upstream_branch_invocation,
};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationRequest, ApplicationResult, ApplicationService,
};
use scriberig_core::EntityId;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

/// Rejects unrelated application operations in the durable report fixture.
struct ReportPersistenceBackend;

impl ApplicationBackend for ReportPersistenceBackend {
    /// Rejects universal operations because the fixture persists session evidence only.
    ///
    /// # Parameters
    /// - `_context`: Caller context, unused by this fixture.
    /// - `_request`: Universal operation, unsupported by this fixture.
    ///
    /// # Errors
    /// Returns an unsupported-operation error for every request.
    ///
    /// # Returns
    /// A successful application result (unreachable because this backend rejects all requests).
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "provider HTTP report fixture".to_owned(),
        })
    }
}

/// Dispatches provider HTTP calls for one exact host-admitted model set.
struct ProviderHttpDispatcher {
    /// Exact provider/model/API tuples selected from the admitted worker.
    models: RwLock<BTreeMap<(String, String, String), rpi_ai::Model>>,
}

impl ProviderHttpDispatcher {
    /// Creates a dispatcher with no admitted models.
    ///
    /// # Returns
    /// A host operation handler that fails closed until models are installed.
    fn new() -> Self {
        Self {
            models: RwLock::new(BTreeMap::new()),
        }
    }

    /// Installs the exact models exposed by the admitted provider package.
    ///
    /// # Parameters
    /// - `models`: Provider models obtained from the worker's allowlisted adapters.
    fn set_models(&self, models: Vec<rpi_ai::Model>) {
        *self
            .models
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = models
            .into_iter()
            .map(|model| {
                (
                    (
                        model.provider.clone(),
                        model.id.clone(),
                        model.api.as_str().to_owned(),
                    ),
                    model,
                )
            })
            .collect();
    }
}

impl PluginSemanticDispatcher for ProviderHttpDispatcher {
    /// Dispatches one host-mediated provider HTTP request.
    ///
    /// # Parameters
    /// - `operation`: Requested host operation.
    /// - `arguments`: Exact provider identity and prepared request.
    ///
    /// # Returns
    /// Host response JSON with the host-generated applied-settings report.
    ///
    /// # Errors
    /// Returns a denial for unknown models, operations, or unsafe requests.
    fn dispatch(&self, operation: &str, arguments: &Value) -> Result<Value, String> {
        if operation != "provider_http_request" {
            return Err("provider fixture operation is not allowed".to_owned());
        }
        let field = |name: &str| {
            arguments
                .get(name)
                .and_then(Value::as_str)
                .ok_or_else(|| "provider fixture identity is invalid".to_owned())
        };
        let key = (
            field("provider")?.to_owned(),
            field("model")?.to_owned(),
            field("api")?.to_owned(),
        );
        let request = arguments
            .get("request")
            .ok_or_else(|| "provider fixture request is missing".to_owned())?;
        let models = self
            .models
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let model = models
            .get(&key)
            .ok_or_else(|| "provider fixture model is not admitted".to_owned())?;
        let request = serde_json::to_string(request)
            .map_err(|_| "provider fixture request is invalid".to_owned())?;
        let response = rpi_extensions::isolated::execute_provider_http_request(model, &request)?;
        serde_json::from_str(&response)
            .map_err(|_| "provider fixture response is invalid".to_owned())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "end-to-end broker scenario asserts the complete worker security boundary"
)]
/// Verifies host RPC sends the exact request and returns complete safe provenance.
#[tokio::test]
async fn isolated_provider_uses_host_http_broker_with_direct_network_denied() {
    let launcher = sandboxed_launcher().expect("runtime-verified sandbox launcher");
    let plugin = provider_plugin_artifact();
    let protected_data = tempfile::tempdir().expect("protected fixture directory");
    let protected_roots = vec![
        protected_data
            .path()
            .canonicalize()
            .expect("canonical protected root"),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider");
    let address = listener.local_addr().expect("fake provider address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept broker request");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .expect("set request read timeout");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).expect("read broker request");
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or_default();
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        let response = b"{\"answer\":\"host-mediated fixture completed\"}";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            response.len()
        )
        .expect("write response headers");
        stream.write_all(response).expect("write response body");
        request
    });
    let dispatcher = Arc::new(ProviderHttpDispatcher::new());
    let worker = start_fixture_worker_with_environment(
        &plugin,
        &protected_roots,
        launcher,
        BTreeSet::from(["provider_http_request".to_owned()]),
        dispatcher.clone(),
        BTreeMap::from([(
            "SCRIBE_FIXTURE_HTTP_ENDPOINT".to_owned(),
            format!("http://{address}"),
        )]),
    );
    let providers = worker
        .provider_adapters(&BTreeSet::from([
            "scriberig-fixture-broker-provider".to_owned()
        ]))
        .expect("discover only the host-mediated fixture provider");
    assert_eq!(providers.len(), 1);
    dispatcher.set_models(providers[0].models().to_vec());
    let model = providers[0].models()[0].clone();
    let context =
        ContextCompiler::new(ContextPolicy::default()).compile(&BranchExecutionContext::new(
            "provider-http-session",
            "provider-http-branch",
            "host-mediated provider HTTP regression",
            "fixture-prompt-sentinel",
        ));
    let invocation = UpstreamBranchInvocation {
        branch: EntityId::from_bytes([71; 16]),
        provider: providers[0].clone() as Arc<dyn Provider>,
        model,
        request_secret: None,
        bearer: false,
        applied_settings: None,
        provider_profile: None,
        thinking_level: ThinkingLevel::Off,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: None,
    };
    let result = run_upstream_branch_invocation(
        invocation,
        "fixture-prompt-sentinel",
        CancellationToken::new(),
        None,
    )
    .await
    .expect("ScribeRig branch invocation through host-mediated provider HTTP");
    assert_eq!(result.output, "host-mediated fixture completed");
    let request = server.join().expect("fake provider server completes");
    let request_text = String::from_utf8_lossy(&request);
    assert!(request_text.contains("authorization: Bearer fixture-credential-sentinel"));
    assert!(request_text.contains("content-type: application/json"));
    assert!(request_text.contains("user-agent: rpi-provider-worker/1"));
    assert!(request_text.contains("fixture-prompt-sentinel"));
    assert!(worker.sandbox_attestation().network);
    let projection = result
        .model_requests
        .first()
        .expect("ScribeRig captured provider request");
    let report = projection
        .applied_settings
        .as_ref()
        .expect("host-generated report captured in request projection");
    assert_eq!(report["complete"], true);
    assert_eq!(report["schemaVersion"], "rpi-applied-request-settings-v5");
    assert!(report["requestEffectSha256"].as_str().is_some());
    assert!(report["requestBodySha256"].as_str().is_some());
    assert!(report["requestHeaderSha256"].as_str().is_some());
    assert_eq!(report["omittedCategories"], serde_json::json!([]));
    let safe_report = serde_json::to_string(report).expect("encode safe report");
    assert!(!safe_report.contains("fixture-credential-sentinel"));
    assert!(!safe_report.contains("fixture-prompt-sentinel"));
    let session = EntityId::from_bytes([72; 16]);
    let database = tempfile::tempdir().expect("durable report directory");
    let path = database.path().join("session.sqlite");
    let store: Arc<dyn SessionStore> =
        Arc::new(SqliteSessionStore::open(&path).expect("open report session store"));
    let application =
        ApplicationService::new(ReportPersistenceBackend).with_session_store(Arc::clone(&store));
    let handle = application
        .start_local_agent(&AgentCommandRequest {
            session,
            branch: None,
            anchor: None,
            prompt: "fixture-prompt-sentinel".to_owned(),
            selection: None,
        })
        .expect("start report persistence session");
    application
        .complete_local_agent_with_evidence(
            &handle,
            &result.output,
            Vec::new(),
            Vec::new(),
            result.model_requests.clone(),
        )
        .expect("persist captured provider request through the application service");
    let durable = store
        .load(session)
        .expect("load persisted request report")
        .expect("durable report session exists");
    let durable_report = durable
        .branches
        .iter()
        .flat_map(|branch| &branch.model_invocations)
        .find_map(|invocation| {
            invocation
                .request
                .as_ref()
                .and_then(|projection| projection.applied_settings.as_ref())
        })
        .expect("broker report survives session persistence");
    assert_eq!(durable_report, report);
    let durable_report_json =
        serde_json::to_string(durable_report).expect("encode persisted applied settings");
    assert!(!durable_report_json.contains("fixture-credential-sentinel"));
    assert!(!durable_report_json.contains("fixture-prompt-sentinel"));
    let durable_json = serde_json::to_string(&durable).expect("encode durable session");
    assert!(!durable_json.contains("fixture-credential-sentinel"));
}

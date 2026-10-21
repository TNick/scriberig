//! Verifies project semantics through the real HTTP client/server boundary.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_app::{
    ApplicationBackendComposition, ApplicationService as AppService, StoreInspectionBackend,
    StoreOperationProvider,
};
use scriberig_client::Client;
use scriberig_project::ProjectService;
use scriberig_server::http::HttpState;
use scriberig_server::{InProcessProjectTransport, InProcessTransport, ensure_fixture};
use scriberig_session_sqlite::SqliteSessionStore;

#[path = "remote_project_conformance/candidate.rs"]
mod candidate_support;
use candidate_support::exercise_remote_aggregate_candidate;
use scriberig_store_sqlite::SqliteStore;
use serde_json::json;

/// Application composition used by the remote public conformance fixture.
type TestApplication = AppService<
    ApplicationBackendComposition<
        StoreInspectionBackend<Arc<SqliteStore>>,
        StoreOperationProvider<Arc<SqliteStore>>,
    >,
>;

/// Creates the deterministic Rust project used by remote conformance.
///
/// - `label`: Unique fixture label.
///
/// Temporary project root and its durable store.
/// # Returns
/// The operation result.
fn fixture(label: &str) -> (PathBuf, Arc<SqliteStore>) {
    let root = std::env::temp_dir().join(format!(
        "scriberig-remote-conformance-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"remote_golden\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    fs::write(
        root.join("src/lib.rs"),
        "pub mod math { pub struct Calculator; impl Calculator { pub fn add(&self, left: i32, right: i32) -> i32 { left + right } } }\n",
    )
    .expect("source");
    let store =
        Arc::new(SqliteStore::open(root.with_extension("scriberig.sqlite")).expect("database"));
    (root, store)
}

/// Builds the same application composition used by the live server.
///
/// - `store`: Durable project store shared by the composition.
/// - `sessions`: Durable session store shared by the composition.
///
/// HTTP state using the production application composition.
/// # Returns
/// The operation result.
fn state(store: Arc<SqliteStore>, sessions: Arc<SqliteSessionStore>) -> HttpState<TestApplication> {
    let fixture = ensure_fixture(&store).expect("development fixture");
    HttpState {
        service: application(Arc::clone(&store), sessions),
        principal: scriberig_app::Principal::new("test-user"),
        fixture,
        allowed_origin: None,
        project_store: Some(store),
        allowed_project_roots: Vec::new(),
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
    }
}

/// Builds shared application operations over project and session stores.
///
/// - `store`: Durable project and application store.
/// - `sessions`: Durable provider-neutral session store.
///
/// The application composition used by embedded and remote clients.
/// # Returns
/// The operation result.
fn application(store: Arc<SqliteStore>, sessions: Arc<SqliteSessionStore>) -> TestApplication {
    let read = StoreInspectionBackend::new(Arc::clone(&store));
    let operations = StoreOperationProvider::new(store);
    let backend = ApplicationBackendComposition::new(read, operations);
    AppService::new(backend).with_session_store(sessions)
}

/// GOLDEN-CONFORMANCE-01..07 prove remote project operations match embedded data.
#[test]
fn golden_conformance_remote_public_project_path() {
    let (root, store) = fixture("public");
    let embedded = ProjectService::new(Arc::clone(&store));
    let scan = embedded
        .scan(Some("remote".to_owned()), &root)
        .expect("scan");
    let imported = embedded.import(scan.project.id).expect("import");
    let entity = embedded
        .symbols(scan.project.id)
        .expect("embedded symbols")
        .into_iter()
        .find(|symbol| symbol.name.as_deref() == Some("add"))
        .expect("add")
        .entity;
    let source = fs::read_to_string(root.join("src/lib.rs")).expect("source");
    let sessions = Arc::new(SqliteSessionStore::open_in_memory().expect("session store"));
    let state = Arc::new(state(Arc::clone(&store), Arc::clone(&sessions)));
    let client = Client::new(InProcessProjectTransport::new(state));
    let session_client = Client::new(InProcessTransport::new(application(
        Arc::clone(&store),
        Arc::clone(&sessions),
    )));

    let workspace = client
        .project("workspace", json!({"project": scan.project.id.to_string()}))
        .expect("remote workspace");
    assert_eq!(workspace["snapshot"], imported.snapshot.to_string());
    assert_eq!(workspace["project"]["id"], scan.project.id.to_string());

    let symbols = client
        .project("symbols", json!({"project": scan.project.id.to_string()}))
        .expect("remote symbols");
    let remote_symbol = symbols["symbols"]
        .as_array()
        .expect("symbol list")
        .iter()
        .find(|symbol| symbol["name"] == "add")
        .expect("remote add");
    assert_eq!(remote_symbol["entityId"], entity.to_string());
    assert_eq!(remote_symbol["snapshotId"], imported.snapshot.to_string());

    let projection = client
        .project(
            "projection",
            json!({"project": scan.project.id.to_string(), "entity": entity.to_string()}),
        )
        .expect("remote projection");
    assert_eq!(projection["path"], "src/lib.rs");
    assert_eq!(projection["text"], source);

    let (aggregate_candidate, aggregate_successor) = exercise_remote_aggregate_candidate(
        &client,
        &root,
        scan.project.id,
        imported.snapshot,
        &source,
    );
    assert_public_transition(
        &client,
        scan.project.id,
        imported.snapshot,
        &aggregate_candidate,
        &aggregate_successor,
    );
    candidate_support::assert_public_branch_result(
        &session_client,
        &sessions,
        &aggregate_candidate,
        &aggregate_successor,
    );
}

/// Verifies public workspace, history, and candidate references describe one transition.
///
/// - `client`: Public project client.
/// - `project`: Stable project identity.
/// - `base`: Exact candidate base snapshot.
/// - `candidate`: Canonical project candidate identity.
/// - `successor`: Applied immutable snapshot identity.
fn assert_public_transition(
    client: &Client<InProcessProjectTransport<TestApplication>>,
    project: scriberig_core::ProjectId,
    base: scriberig_core::SnapshotId,
    candidate: &str,
    successor: &str,
) {
    let workspace = client
        .project("workspace", json!({"project": project.to_string()}))
        .expect("public successor workspace");
    assert_eq!(workspace["snapshot"], successor);
    let history = client
        .project("history", json!({"project": project.to_string()}))
        .expect("public history");
    let snapshots = history["snapshots"].as_array().expect("snapshot lineage");
    assert!(snapshots.iter().any(|value| value == &base.to_string()));
    assert!(snapshots.iter().any(|value| value == successor));
    assert!(
        history["transitions"]
            .as_array()
            .is_some_and(|transitions| {
                transitions.iter().any(|transition| {
                    transition["base"] == base.to_string()
                        && transition["result"] == successor
                        && transition["candidate"] == candidate
                })
            })
    );
    let detail = client
        .project("candidate", json!({"candidate": candidate}))
        .expect("public candidate detail");
    assert_eq!(detail["base"], base.to_string());
    assert_eq!(detail["validation"], "passed");
    assert_eq!(detail["effectPaths"].as_array().expect("effects").len(), 2);
}

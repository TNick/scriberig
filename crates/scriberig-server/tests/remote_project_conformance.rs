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
use scriberig_server::{InProcessProjectTransport, ensure_fixture};
use scriberig_session_sqlite::SqliteSessionStore;
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
/// # Parameters
/// - `label`: Unique fixture label.
///
/// # Returns
/// Temporary project root and its durable store.
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
/// # Parameters
/// - `store`: Durable project store shared by the composition.
///
/// # Returns
/// HTTP state using the production application composition.
fn state(store: Arc<SqliteStore>) -> HttpState<TestApplication> {
    let fixture = ensure_fixture(&store).expect("development fixture");
    let read = StoreInspectionBackend::new(Arc::clone(&store));
    let operations = StoreOperationProvider::new(Arc::clone(&store));
    let backend = ApplicationBackendComposition::new(read, operations);
    let sessions = Arc::new(SqliteSessionStore::open_in_memory().expect("session store"));
    HttpState {
        service: AppService::new(backend).with_session_store(sessions),
        fixture,
        allowed_origin: None,
        project_store: Some(store),
        allowed_project_roots: Vec::new(),
    }
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
    let state = Arc::new(state(Arc::clone(&store)));
    let client = Client::new(InProcessProjectTransport::new(state));

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

    let aggregate_successor = exercise_remote_aggregate_candidate(
        &client,
        &root,
        scan.project.id,
        imported.snapshot,
        &source,
    );
    let aggregate_source = fs::read_to_string(root.join("src/lib.rs")).expect("aggregate source");

    let candidate = client
        .project(
            "propose",
            json!({
                "project": scan.project.id.to_string(),
                "entity": entity.to_string(),
                "text": aggregate_source
            }),
        )
        .expect("remote candidate");
    assert_eq!(candidate["base"], aggregate_successor);
    assert_eq!(candidate["validation"], "passed");
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("unchanged source"),
        aggregate_source
    );

    let applied = client
        .project("apply", json!({"candidate": candidate["candidate"]}))
        .expect("remote apply");
    assert_eq!(applied["materialization"], "applied");
    assert_ne!(applied["successor"], imported.snapshot.to_string());
    assert!(
        fs::read_to_string(root.join("src/lib.rs"))
            .expect("materialized source")
            .contains("left.saturating_add(right)")
    );
}

/// Verifies aggregate proposal, detail, and validation through the remote client.
///
/// # Parameters
/// - `client`: Real protocol client for the in-process HTTP server.
/// - `root`: Live project root that must remain unchanged.
/// - `project`: Exact project identity.
/// - `snapshot`: Exact candidate base snapshot.
/// - `source`: Base source bytes as UTF-8.
///
/// # Returns
/// Successor snapshot ID after aggregate Apply.
fn exercise_remote_aggregate_candidate(
    client: &Client<InProcessProjectTransport<TestApplication>>,
    root: &std::path::Path,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
    source: &str,
) -> String {
    let aggregate = client
        .project(
            "propose-change",
            json!({
                "project": project.to_string(),
                "base_snapshot": snapshot.to_string(),
                "effects": [
                    {
                        "path": "src/lib.rs",
                        "kind": "update",
                        "content": source.replace("left + right", "left.saturating_add(right)").into_bytes()
                    },
                    {
                        "path": "src/generated.rs",
                        "kind": "create",
                        "content": b"pub fn generated() -> u8 { 1 }\n".to_vec()
                    }
                ]
            }),
        )
        .expect("remote aggregate candidate");
    assert_eq!(aggregate["base"], snapshot.to_string());
    assert_eq!(aggregate["effects"].as_array().expect("effects").len(), 2);
    assert!(!root.join("src/generated.rs").exists());
    let inspected = client
        .project("candidate", json!({"candidate": aggregate["candidate"]}))
        .expect("durable candidate detail");
    assert_eq!(inspected["effects"][0]["path"], "src/generated.rs");
    assert_eq!(
        inspected["effects"][0]["newBytes"],
        json!(b"pub fn generated() -> u8 { 1 }\n".to_vec())
    );
    assert_eq!(inspected["effects"][1]["path"], "src/lib.rs");
    assert_eq!(
        inspected["effects"][1]["oldBytes"],
        json!(source.as_bytes())
    );
    assert_eq!(
        inspected["effects"][1]["newBytes"],
        json!(
            source
                .replace("left + right", "left.saturating_add(right)")
                .into_bytes()
        )
    );
    let validated = client
        .project(
            "validate-candidate",
            json!({"candidate": aggregate["candidate"]}),
        )
        .expect("isolated candidate validation");
    assert_eq!(validated["validation"], "passed");
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("live source unchanged"),
        source
    );
    let applied = client
        .project("apply", json!({"candidate": aggregate["candidate"]}))
        .expect("apply aggregate candidate");
    assert_eq!(applied["materialization"], "applied");
    assert_eq!(applied["paths"].as_array().expect("paths").len(), 2);
    let effect_detail = client
        .project("candidate", json!({"candidate": aggregate["candidate"]}))
        .expect("durable effect detail");
    assert_eq!(
        effect_detail["effectPaths"]
            .as_array()
            .expect("path receipts")
            .len(),
        2
    );
    assert!(
        effect_detail["effectPaths"]
            .as_array()
            .expect("path receipts")
            .iter()
            .all(|receipt| receipt["state"] == "applied")
    );
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("materialized source"),
        source.replace("left + right", "left.saturating_add(right)")
    );
    assert_eq!(
        fs::read(root.join("src/generated.rs")).expect("materialized candidate file"),
        b"pub fn generated() -> u8 { 1 }\n"
    );
    applied["successor"]
        .as_str()
        .expect("successor snapshot")
        .to_owned()
}

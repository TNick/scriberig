//! Verifies provider document tools create candidates through product composition.

use std::{collections::BTreeSet, fs, sync::Arc};

use scriberig_agent_core::Tool as _;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Verifies product catalog dispatch creates an exact-base paragraph candidate.
#[test]
fn product_document_tool_returns_shared_candidate_without_live_source_write() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-agent-docx-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("project root");
    let path = root.join("report.docx");
    let original = include_bytes!("../../../tests/fixtures/rich-project/report.docx");
    fs::write(&path, original).expect("copy DOCX fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan DOCX");
    let imported = projects.import(scan.project.id).expect("import DOCX");
    let paragraph = store
        .project_entities(scan.project.id)
        .expect("entity ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(imported.snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let value: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (value["runs"][0]["text"] == "Existing site area").then_some(entity.entity)
        })
        .expect("semantic paragraph");
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects.clone());
    let context = ApplicationContext {
        snapshot: Some(imported.snapshot),
        capabilities: CapabilityContext::with_grants([
            Capability {
                scope: CapabilityScope::Project,
                actions: BTreeSet::from([CapabilityAction::Custom(
                    "document.replace_paragraph".to_owned(),
                )]),
                constraints: CapabilityConstraints::unrestricted(),
            },
            Capability {
                scope: CapabilityScope::Custom(format!("project:{}", scan.project.id)),
                actions: BTreeSet::from([CapabilityAction::Propose]),
                constraints: CapabilityConstraints {
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            },
        ]),
        ..ApplicationContext::default()
    };
    let result = LocalAgentTool::new(&service)
        .invoke_named(
            &context,
            "document.replace_paragraph",
            &serde_json::json!({
                "project": scan.project.id.to_string(),
                "paragraph": paragraph.to_string(),
                "replacement": "Proposed site area"
            }),
        )
        .expect("provider document candidate");
    assert_eq!(result.entity, paragraph);
    assert_eq!(result.snapshot, Some(imported.snapshot));
    assert_eq!(result.object_type.as_deref(), Some("document.candidate"));
    assert_eq!(result.handles.len(), 1);
    assert_eq!(
        fs::read(&path).expect("live source remains unchanged"),
        original
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project root");
}

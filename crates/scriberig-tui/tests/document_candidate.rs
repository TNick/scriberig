//! Verifies the TUI document command creates a real candidate through the app.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, OperationKind, StoreInspectionBackend,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;
use scriberig_tui::action::Action;
use scriberig_tui::effect::Effect;
use scriberig_tui::inspector::LoadState;
use scriberig_tui::service::{ApplicationOperationAdapter, TuiService, execute_effect};
use scriberig_tui::state::{TuiState, reduce};

/// Creates one exact project scope with document inspection and proposal grants.
///
/// # Parameters
/// - `project`: Project that owns the checked-in DOCX fixture.
/// - `snapshot`: Exact imported base snapshot.
///
/// # Returns
/// Context authorized to inspect the document and propose its paragraph edit.
fn document_context(project: ProjectId, snapshot: SnapshotId) -> ApplicationContext {
    let scope = CapabilityScope::Custom(format!("project:{project}"));
    let inspect = Capability {
        scope: scope.clone(),
        actions: BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints {
            operation: Some(OperationKind::Inspect),
            ..CapabilityConstraints::unrestricted()
        },
    };
    let propose = Capability {
        scope,
        actions: BTreeSet::from([CapabilityAction::Propose]),
        constraints: CapabilityConstraints {
            operation: Some(OperationKind::DomainOperation),
            ..CapabilityConstraints::unrestricted()
        },
    };
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([inspect, propose]),
        ..ApplicationContext::default()
    }
}

/// Runs the loaded TUI paragraph command through a real app candidate operation.
#[test]
fn document_command_creates_candidate_for_shared_docx_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/document-paragraph-edit.json"
    ))
    .expect("shared web/TUI DOCX candidate fixture");
    let root = std::env::temp_dir().join(format!("scriberig-tui-docx-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create isolated project root");
    let source = fs::read("../../tests/fixtures/rich-project/report.docx")
        .expect("checked-in rich project DOCX fixture");
    fs::write(root.join("report.docx"), &source).expect("copy DOCX fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan DOCX fixture");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import DOCX fixture");
    let application = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects.clone());
    let adapter = ApplicationOperationAdapter::new(application).with_project_workflow(projects);
    let context = document_context(project, imported.snapshot);
    let document = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root")
        .entity;
    let tree = adapter
        .inspect_document(
            &context,
            &scriberig_app::DocumentTreeQuery {
                project,
                snapshot: imported.snapshot,
                document,
            },
        )
        .expect("inspect imported document");
    let paragraph = fixture_paragraph_id(&fixture, &tree.sections);
    let mut state = TuiState::default();
    state.agent_selection = Some(scriberig_app::AgentSelection {
        project: Some(project),
        root: document,
        entity: document,
        snapshot: imported.snapshot,
    });
    state.document_root = Some(document);
    state.document_snapshot = Some(imported.snapshot);
    state.document_tree = LoadState::Ready(tree);
    state.conversation_composer.draft = format!(
        "/document-replace {paragraph} {}",
        fixture["replacement"]
            .as_str()
            .expect("fixture replacement")
    );

    let effects = reduce(&mut state, Action::InputSubmitted);
    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeDocumentParagraphEdit { .. }]
    ));
    let action = execute_effect(
        &adapter,
        &context,
        effects.into_iter().next().expect("edit effect"),
    );

    let Action::DocumentEditProposed { candidate, .. } = action else {
        panic!("TUI command did not create a document candidate: {action:?}");
    };
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.path, "report.docx");
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Pending
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("live source remains unchanged"),
        source
    );

    drop(adapter);
    drop(store);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&database);
}

/// Resolves the fixture paragraph's semantic identity from an imported tree.
///
/// # Parameters
/// - `fixture`: Shared document-edit fixture containing expected paragraph text.
/// - `sections`: Imported document sections returned by the application adapter.
///
/// # Returns
/// Semantic identity of the matching paragraph.
fn fixture_paragraph_id(
    fixture: &serde_json::Value,
    sections: &[scriberig_protocol::DocumentSectionView],
) -> EntityId {
    let paragraph_text = fixture["paragraphText"]
        .as_str()
        .expect("fixture paragraph text");
    sections
        .iter()
        .flat_map(|section| &section.blocks)
        .find_map(|block| match block {
            scriberig_protocol::DocumentBlockView::Paragraph { paragraph }
                if paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>()
                    == paragraph_text =>
            {
                paragraph.id.parse::<EntityId>().ok()
            }
            _ => None,
        })
        .expect("fixture paragraph semantic identity")
}

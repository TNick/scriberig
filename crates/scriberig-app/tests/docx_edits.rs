//! Verifies authorized document edits produce candidates through the app boundary.

use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Write};
use std::sync::Arc;

use scriberig_agent_core::Tool;
use scriberig_app::agent_tools::{
    DOCUMENT_PARAGRAPH_EDIT_TOOL, DOCUMENT_PARAGRAPH_INSERT_TOOL, DOCUMENT_TABLE_CELL_EDIT_TOOL,
    DocumentParagraphEditTool, DocumentParagraphInsertTool, DocumentTableCellEditTool,
};
use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
};
use scriberig_project::ProjectService;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

/// Creates a project-scoped Propose context for one exact snapshot.
///
/// # Parameters
/// - `project`: Project identity to authorize.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Scoped application context.
fn propose_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

/// Creates a project-scoped Inspect context for one exact snapshot.
///
/// # Parameters
/// - `project`: Project identity to authorize.
/// - `snapshot`: Exact immutable snapshot to inspect.
///
/// # Returns
/// Scoped application context.
fn inspect_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::Inspect),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "This application integration test verifies authorization, candidate generation, and unchanged source bytes in one flow."
)]
/// Verifies document candidate creation requires Propose authority and leaves source unchanged.
#[test]
fn document_paragraph_edit_requires_propose_grant_and_keeps_live_bytes_unchanged() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/document-paragraph-edit.json"
    ))
    .expect("shared web/TUI paragraph edit fixture");
    let paragraph_text = fixture["paragraphText"]
        .as_str()
        .expect("fixture paragraph text");
    let replacement = fixture["replacement"]
        .as_str()
        .expect("fixture replacement");
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-docx-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    let document_xml = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{paragraph_text}</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>parcel 124</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#
    );
    archive
        .write_all(document_xml.as_bytes())
        .expect("document XML");
    let original = archive.finish().expect("finish DOCX").into_inner();
    let path = root.join("report.docx");
    fs::write(&path, &original).expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan DOCX");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import DOCX");
    let paragraph = store
        .project_entities(project)
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
            (value["runs"][0]["text"] == paragraph_text).then_some(entity.entity)
        })
        .expect("semantic paragraph");
    let cell = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.table_cell")
        .expect("semantic table cell")
        .entity;
    let application = ApplicationService::new(()).with_project_service(projects);
    let request = ApplicationContext::default().with_snapshot(imported.snapshot);
    let document = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root");
    assert!(matches!(
        application.inspect_document(&request, project, imported.snapshot, document.entity),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let tree = application
        .inspect_document(
            &inspect_context(project, imported.snapshot),
            project,
            imported.snapshot,
            document.entity,
        )
        .expect("authorized document structure");
    assert_eq!(tree.document, document.entity);
    assert_eq!(tree.sections[0].blocks.len(), 2);
    assert!(matches!(
        application.find_document_paragraphs(
            &request,
            &scriberig_app::DocumentParagraphSearchRequest {
                project,
                snapshot: imported.snapshot,
                document: document.entity,
                query: "site".into(),
                offset: 0,
                limit: 10,
            },
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let matches = application
        .find_document_paragraphs(
            &inspect_context(project, imported.snapshot),
            &scriberig_app::DocumentParagraphSearchRequest {
                project,
                snapshot: imported.snapshot,
                document: document.entity,
                query: paragraph_text.to_ascii_uppercase(),
                offset: 0,
                limit: 10,
            },
        )
        .expect("authorized paragraph search");
    assert_eq!(matches.total, 1);
    assert_eq!(matches.matches[0].text, paragraph_text);
    assert!(matches!(
        application.propose_document_paragraph_edit(&request, project, paragraph, replacement),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let candidate = application
        .propose_document_paragraph_edit(
            &propose_context(project, imported.snapshot),
            project,
            paragraph,
            replacement,
        )
        .expect("authorized document candidate");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.path, "report.docx");
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Pending
    );
    let section = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.section")
        .expect("document section");
    assert!(matches!(
        application.propose_document_paragraph_insert(
            &request,
            project,
            section.entity,
            "Untrusted append",
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let inserted = application
        .propose_document_paragraph_insert(
            &propose_context(project, imported.snapshot),
            project,
            section.entity,
            "Appended section text",
        )
        .expect("authorized paragraph insertion candidate");
    assert_eq!(inserted.project, project);
    assert_eq!(inserted.base_snapshot, imported.snapshot);
    assert_eq!(inserted.path, "report.docx");
    assert_eq!(
        inserted.validation,
        scriberig_project::CandidateValidation::Pending
    );

    let agent_tool = DocumentParagraphEditTool::new(
        ApplicationService::new(()).with_project_service(ProjectService::new(Arc::clone(&store))),
        project,
    );
    let tool_arguments = serde_json::json!({
        "paragraph": paragraph.to_string(),
        "replacement": "Agent proposed site area",
    });
    assert!(matches!(
        agent_tool.invoke_named(
            &ApplicationContext::default().with_snapshot(imported.snapshot),
            DOCUMENT_PARAGRAPH_EDIT_TOOL,
            &tool_arguments,
        ),
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let agent_candidate = agent_tool
        .invoke_named(
            &propose_context(project, imported.snapshot),
            DOCUMENT_PARAGRAPH_EDIT_TOOL,
            &tool_arguments,
        )
        .expect("authorized agent candidate");
    assert_eq!(agent_candidate.entity, paragraph);
    assert_eq!(agent_candidate.snapshot, Some(imported.snapshot));
    assert_eq!(agent_candidate.handles.len(), 1);
    let cell_tool = DocumentTableCellEditTool::new(
        ApplicationService::new(()).with_project_service(ProjectService::new(Arc::clone(&store))),
        project,
    );
    let cell_arguments = serde_json::json!({
        "cell": cell.to_string(),
        "replacement": "parcel 125",
    });
    assert!(matches!(
        cell_tool.invoke_named(
            &ApplicationContext::default().with_snapshot(imported.snapshot),
            DOCUMENT_TABLE_CELL_EDIT_TOOL,
            &cell_arguments,
        ),
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let cell_candidate = cell_tool
        .invoke_named(
            &propose_context(project, imported.snapshot),
            DOCUMENT_TABLE_CELL_EDIT_TOOL,
            &cell_arguments,
        )
        .expect("authorized table-cell candidate");
    assert_eq!(cell_candidate.entity, cell);
    assert_eq!(cell_candidate.snapshot, Some(imported.snapshot));
    assert_eq!(cell_candidate.handles.len(), 1);
    let insert_tool = DocumentParagraphInsertTool::new(
        ApplicationService::new(()).with_project_service(ProjectService::new(Arc::clone(&store))),
        project,
    );
    let insert_arguments = serde_json::json!({
        "section": section.entity.to_string(),
        "text": "Agent appended paragraph",
    });
    assert!(matches!(
        insert_tool.invoke_named(
            &ApplicationContext::default().with_snapshot(imported.snapshot),
            DOCUMENT_PARAGRAPH_INSERT_TOOL,
            &insert_arguments,
        ),
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let insert_candidate = insert_tool
        .invoke_named(
            &propose_context(project, imported.snapshot),
            DOCUMENT_PARAGRAPH_INSERT_TOOL,
            &insert_arguments,
        )
        .expect("authorized agent insertion candidate");
    assert_eq!(insert_candidate.entity, section.entity);
    assert_eq!(insert_candidate.snapshot, Some(imported.snapshot));
    assert_eq!(insert_candidate.handles.len(), 1);
    assert_eq!(fs::read(&path).expect("live DOCX"), original);

    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

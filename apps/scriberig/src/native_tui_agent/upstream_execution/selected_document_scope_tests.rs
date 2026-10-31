//! Verifies selected rich-document tools follow exact target and fidelity grants.

use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Write};
use std::sync::Arc;

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityScope, OperationKind,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

use super::for_selected_geo_entity;

/// Exposes DOCX paragraph mutation only for the selected editable target.
#[test]
fn document_edit_tool_requires_selected_editable_paragraph_and_exact_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/report.docx");
    fs::copy(fixture, directory.path().join("report.docx")).expect("copy DOCX fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan DOCX project")
        .project
        .id;
    let snapshot = projects
        .import(project)
        .expect("import DOCX project")
        .snapshot;
    let paragraph = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "document.paragraph")
        .expect("document paragraph");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("document.replace_paragraph".to_owned()),
                CapabilityAction::Custom("document.update_table_cell".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: paragraph.entity.to_string(),
        selection: Some(paragraph.entity.to_string()),
    };

    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        visible
            .iter()
            .any(|name| name == "document.replace_paragraph")
    );
    assert!(
        !visible
            .iter()
            .any(|name| name == "document.update_table_cell")
    );
    assert!(scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(paragraph.entity),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Custom(format!("project:{project}")),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));

    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    assert!(
        !crate::child_runtime::inspect_catalog()
            .descriptors_for(&unselected.capabilities)
            .iter()
            .any(|descriptor| descriptor.definition.name.starts_with("document."))
    );

    let stale = ProjectSemanticScope {
        snapshot: EntityId::new().to_string(),
        ..selected
    };
    let stale_context = for_selected_geo_entity(&context, Some(&stale), &store, &projects);
    assert!(
        !crate::child_runtime::inspect_catalog()
            .descriptors_for(&stale_context.capabilities)
            .iter()
            .any(|descriptor| descriptor.definition.name.starts_with("document."))
    );
}

/// Exposes a DOCX table-cell edit only when that exact cell is selected.
#[test]
fn document_cell_edit_tool_requires_selected_editable_cell_and_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("DOCX document part");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell value</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#)
        .expect("write DOCX document XML");
    fs::write(
        directory.path().join("table.docx"),
        archive.finish().expect("finish DOCX").into_inner(),
    )
    .expect("write DOCX fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan DOCX project")
        .project
        .id;
    let snapshot = projects
        .import(project)
        .expect("import DOCX project")
        .snapshot;
    let cell = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "document.table_cell")
        .expect("document table cell");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("document.replace_paragraph".to_owned()),
                CapabilityAction::Custom("document.update_table_cell".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = scriberig_agent_runtime::context::ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: cell.entity.to_string(),
        selection: Some(cell.entity.to_string()),
    };
    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        visible
            .iter()
            .any(|name| name == "document.update_table_cell")
    );
    assert!(
        !visible
            .iter()
            .any(|name| name == "document.replace_paragraph")
    );
    assert!(scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(cell.entity),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
}

//! Verifies exact-snapshot document outline and semantic-part navigation.

use super::*;

/// Routes `/document` through the selected project, snapshot, and document identity.
#[test]
fn document_command_requests_exact_selected_tree() {
    let project = scriberig_core::ProjectId::from_bytes([61; 16]);
    let document = scriberig_core::EntityId::from_bytes([62; 16]);
    let snapshot = scriberig_core::SnapshotId::new([63; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: document,
            entity: document,
            snapshot,
        }),
        selection: Some(crate::selection::EntitySelection {
            entity: document,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/document".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectDocument { request_id: 1, request }]
            if request.project == project
                && request.snapshot == snapshot
                && request.document == document
    ));
    assert!(matches!(
        state.document_tree,
        crate::inspector::LoadState::Loading { request_id: 1 }
    ));
}

/// Selects a paragraph only when its semantic identity exists in the loaded tree.
#[test]
fn document_part_selection_is_exact_and_keeps_document_root() {
    let document = scriberig_core::EntityId::from_bytes([64; 16]);
    let paragraph = scriberig_core::EntityId::from_bytes([65; 16]);
    let snapshot = scriberig_core::SnapshotId::new([66; 32]);
    let tree = scriberig_protocol::DocumentInspectResponse {
        project: "project-1".to_owned(),
        snapshot: snapshot.to_string(),
        document: document.to_string(),
        artifact_capability: None,
        styles: Vec::new(),
        sections: vec![scriberig_protocol::DocumentSectionView {
            id: scriberig_core::EntityId::from_bytes([67; 16]).to_string(),
            blocks: vec![scriberig_protocol::DocumentBlockView::Paragraph {
                paragraph: scriberig_protocol::DocumentParagraphView {
                    id: paragraph.to_string(),
                    runs: Vec::new(),
                    style: None,
                },
            }],
        }],
    };
    let mut state = TuiState {
        document_tree: crate::inspector::LoadState::Ready(tree),
        document_root: Some(document),
        document_snapshot: Some(snapshot),
        ..TuiState::default()
    };

    reduce(
        &mut state,
        Action::DocumentPartSelected { entity: paragraph },
    );

    assert_eq!(state.document_root, Some(document));
    assert_eq!(
        state.selection,
        Some(crate::selection::EntitySelection {
            entity: paragraph,
            revision: None,
            snapshot: Some(snapshot),
        })
    );
}

/// Reads another bounded outline window without repeating the remote tree request.
#[test]
fn document_command_pages_the_loaded_tree_locally() {
    let project = scriberig_core::ProjectId::from_bytes([68; 16]);
    let document = scriberig_core::EntityId::from_bytes([69; 16]);
    let snapshot = scriberig_core::SnapshotId::new([70; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: document,
            entity: document,
            snapshot,
        }),
        selection: Some(crate::selection::EntitySelection {
            entity: document,
            revision: None,
            snapshot: Some(snapshot),
        }),
        document_tree: crate::inspector::LoadState::Ready(
            scriberig_protocol::DocumentInspectResponse {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                document: document.to_string(),
                artifact_capability: None,
                styles: Vec::new(),
                sections: Vec::new(),
            },
        ),
        document_root: Some(document),
        document_snapshot: Some(snapshot),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/document 80".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.document_offset, 80);
}

/// Routes exact paragraph replacement through the shared candidate API.
#[test]
fn document_replace_command_requests_a_candidate_for_the_loaded_paragraph() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/document-paragraph-edit.json"
    ))
    .expect("shared web/TUI paragraph edit fixture");
    let project = fixture["project"]
        .as_str()
        .expect("fixture project")
        .parse::<scriberig_core::ProjectId>()
        .expect("valid project identity");
    let document = scriberig_core::EntityId::from_bytes([72; 16]);
    let paragraph = fixture["paragraph"]
        .as_str()
        .expect("fixture paragraph")
        .parse::<scriberig_core::EntityId>()
        .expect("valid paragraph identity");
    let snapshot = fixture["snapshot"]
        .as_str()
        .expect("fixture snapshot")
        .parse::<scriberig_core::SnapshotId>()
        .expect("valid snapshot identity");
    let tree = document_tree(project, document, paragraph, snapshot);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: document,
            entity: document,
            snapshot,
        }),
        document_root: Some(document),
        document_snapshot: Some(snapshot),
        document_tree: crate::inspector::LoadState::Ready(tree),
        ..TuiState::default()
    };
    state.conversation_composer.draft = format!(
        "/document-replace {paragraph} {}",
        fixture["replacement"]
            .as_str()
            .expect("fixture replacement")
    );

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeDocumentParagraphEdit { request_id: 1, request }]
            if request.project == project
                && request.snapshot == snapshot
                && request.paragraph == paragraph
                && request.replacement == fixture["replacement"].as_str().unwrap()
    ));
}

/// Routes exact table-cell replacement through the shared candidate API.
#[test]
fn document_cell_replace_command_requests_candidate_for_loaded_cell() {
    let project = scriberig_core::ProjectId::from_bytes([78; 16]);
    let document = scriberig_core::EntityId::from_bytes([79; 16]);
    let cell = scriberig_core::EntityId::from_bytes([80; 16]);
    let snapshot = scriberig_core::SnapshotId::new([81; 32]);
    let mut tree = document_tree(
        project,
        document,
        scriberig_core::EntityId::from_bytes([82; 16]),
        snapshot,
    );
    tree.sections[0]
        .blocks
        .push(scriberig_protocol::DocumentBlockView::Table {
            table: scriberig_protocol::DocumentTableView {
                id: scriberig_core::EntityId::from_bytes([83; 16]).to_string(),
                rows: vec![scriberig_protocol::DocumentRowView {
                    id: scriberig_core::EntityId::from_bytes([84; 16]).to_string(),
                    cells: vec![scriberig_protocol::DocumentCellView {
                        id: cell.to_string(),
                        blocks: Vec::new(),
                    }],
                }],
            },
        });
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: document,
            entity: document,
            snapshot,
        }),
        document_root: Some(document),
        document_snapshot: Some(snapshot),
        document_tree: crate::inspector::LoadState::Ready(tree),
        ..TuiState::default()
    };
    state.conversation_composer.draft = format!("/document-cell-replace {cell} parcel 125");
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert!(
        matches!(effects.as_slice(), [Effect::ProposeDocumentTableCellEdit { request_id: 1, request }]
        if request.project == project && request.snapshot == snapshot && request.cell == cell && request.replacement == "parcel 125")
    );
}

/// Opens the existing Changes review when a document candidate is created.
#[test]
fn document_candidate_opens_shared_review() {
    let snapshot = scriberig_core::SnapshotId::new([75; 32]);
    let mut state = TuiState {
        document_edit: crate::inspector::LoadState::Loading { request_id: 6 },
        next_request_id: 7,
        ..TuiState::default()
    };

    let effects = reduce(
        &mut state,
        Action::DocumentEditProposed {
            request_id: 6,
            candidate: scriberig_app::DocumentParagraphCandidate {
                id: "doc-candidate-1".to_owned(),
                project: scriberig_core::ProjectId::from_bytes([76; 16]),
                base_snapshot: snapshot,
                path: "report.docx".to_owned(),
                validation: scriberig_project::CandidateValidation::Pending,
            },
        },
    );

    assert!(matches!(
        effects.as_slice(),
        [Effect::Apply { request, .. }]
            if request.change == "doc-candidate-1" && request.base_snapshot == snapshot
    ));
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
}

/// Builds one semantic document fixture with an addressable paragraph.
///
/// # Parameters
/// - `project`: Stable project identity for the fixture.
/// - `document`: Semantic document root identity.
/// - `paragraph`: Addressable paragraph identity.
/// - `snapshot`: Exact immutable snapshot context.
///
/// # Returns
/// A single-section document tree containing the supplied paragraph.
fn document_tree(
    project: scriberig_core::ProjectId,
    document: scriberig_core::EntityId,
    paragraph: scriberig_core::EntityId,
    snapshot: scriberig_core::SnapshotId,
) -> scriberig_protocol::DocumentInspectResponse {
    scriberig_protocol::DocumentInspectResponse {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        document: document.to_string(),
        artifact_capability: Some(scriberig_protocol::ArtifactEditCapability::LosslessEditable),
        styles: Vec::new(),
        sections: vec![scriberig_protocol::DocumentSectionView {
            id: scriberig_core::EntityId::from_bytes([77; 16]).to_string(),
            blocks: vec![scriberig_protocol::DocumentBlockView::Paragraph {
                paragraph: scriberig_protocol::DocumentParagraphView {
                    id: paragraph.to_string(),
                    runs: Vec::new(),
                    style: None,
                },
            }],
        }],
    }
}

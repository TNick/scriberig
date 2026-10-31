//! Asserts exact candidate behavior in the mixed rich-artifact integration test.

use std::fs;
use std::io::Read;
use std::path::Path;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_domain_document::Paragraph;
use scriberig_project::{ProjectCandidate, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::{ProjectEntityRecord, SqliteStore};
use zip::ZipArchive;

/// Verifies an opaque DOCX package member survives a mixed-project edit exactly.
///
/// - `root`: Fresh project copy containing the materialized DOCX.
pub(crate) fn assert_docx_opaque_anchor(root: &Path) {
    let package = fs::read(root.join("report.docx")).expect("materialized DOCX package");
    let mut archive = ZipArchive::new(std::io::Cursor::new(package)).expect("open DOCX package");
    let mut member = archive
        .by_name("customXml/opaque.bin")
        .expect("opaque package member");
    let mut bytes = Vec::new();
    member
        .read_to_end(&mut bytes)
        .expect("read opaque package member");
    assert_eq!(bytes, b"rich-project-opaque-docx-anchor\0");
}

/// Checks a candidate records its exact base snapshot and semantic target.
///
/// - `candidate`: Durable candidate returned by the semantic operation.
/// - `base`: Snapshot the operation was authorized to edit.
/// - `target`: Exact semantic entity selected by the caller.
pub(crate) fn assert_candidate_target(
    candidate: &ProjectCandidate,
    base: SnapshotId,
    target: EntityId,
) {
    assert_eq!(candidate.base_snapshot, base);
    assert_eq!(candidate.semantic_target, Some(target));
}

/// Confirms a candidate-only operation leaves one live artifact unchanged.
///
/// - `root`: Fresh mixed-project directory.
/// - `relative_path`: Project-relative artifact path.
/// - `expected`: Exact bytes captured before candidate creation.
pub(crate) fn assert_live_file(root: &Path, relative_path: &str, expected: &[u8]) {
    assert_eq!(
        fs::read(root.join(relative_path)).expect("live artifact bytes"),
        expected
    );
}

/// Verifies a DOCX target paragraph receives a new semantic revision.
///
/// - `service`: Project query service for exact-snapshot reads.
/// - `store`: Revision store resolving immutable paragraph records.
/// - `project`: Project containing the DOCX artifact.
/// - `base`: Snapshot immediately before the DOCX candidate.
/// - `successor`: Snapshot after shared candidate apply.
/// - `paragraph`: Exact semantic paragraph identity targeted by the operation.
pub(crate) fn assert_document_paragraph_change(
    service: &ProjectService,
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    base: SnapshotId,
    successor: SnapshotId,
    paragraph: EntityId,
) {
    let before = paragraph_at(store, base, paragraph);
    let after = paragraph_at(store, successor, paragraph);
    assert_eq!(before.id, after.id, "paragraph identity remains stable");
    assert_eq!(paragraph_text(&before), "Existing site area");
    assert_eq!(paragraph_text(&after), "Proposed site area");
    assert_ne!(
        store
            .resolve(base, paragraph)
            .expect("base paragraph revision lookup"),
        store
            .resolve(successor, paragraph)
            .expect("successor paragraph revision lookup")
    );
    let view = service
        .document_tree(
            project,
            successor,
            document_for_paragraph(store, project, paragraph),
        )
        .expect("read exact successor DOCX tree");
    assert_eq!(view.snapshot, successor);
}

/// Loads one paragraph revision from an immutable snapshot.
///
/// - `store`: Revision store containing the entity history.
/// - `snapshot`: Exact snapshot to resolve.
/// - `paragraph`: Semantic paragraph identity.
///
/// # Returns
/// The paragraph value stored at the requested snapshot.
fn paragraph_at(
    store: &SqliteStore,
    snapshot: SnapshotId,
    paragraph: EntityId,
) -> scriberig_domain_document::Paragraph {
    let revision = store
        .resolve(snapshot, paragraph)
        .expect("paragraph snapshot lookup")
        .expect("paragraph revision exists");
    serde_json::from_slice(
        store
            .get(revision)
            .expect("paragraph revision read")
            .expect("paragraph payload")
            .payload(),
    )
    .expect("decode paragraph semantics")
}

/// Joins authored text runs for compact semantic assertions.
///
/// - `paragraph`: Paragraph whose authored text is inspected.
///
/// # Returns
/// Concatenated visible text in run order.
fn paragraph_text(paragraph: &scriberig_domain_document::Paragraph) -> String {
    paragraph.runs.iter().map(|run| run.text.as_str()).collect()
}

/// Resolves a document root that owns one semantic paragraph.
///
/// - `store`: Project store containing source ownership records.
/// - `project`: Project whose document is inspected.
/// - `paragraph`: Semantic paragraph identity.
///
/// # Returns
/// The semantic document root identity owning the paragraph.
fn document_for_paragraph(
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    paragraph: EntityId,
) -> EntityId {
    let owner = store
        .project_entity(project, paragraph)
        .expect("paragraph ownership lookup")
        .expect("paragraph is owned");
    store
        .project_entities(project)
        .expect("document ownership records")
        .into_iter()
        .find(|entity| {
            entity.kind == "document.root" && entity.relative_path == owner.relative_path
        })
        .expect("document root owns target paragraph")
        .entity
}

/// Resolves a named worksheet identity at one exact project snapshot.
///
/// - `store`: Persisted project store.
/// - `entities`: Ownership records from the same project scan.
/// - `snapshot`: Exact immutable snapshot used for worksheet metadata.
/// - `name`: Authored worksheet name to resolve.
///
/// # Returns
/// Stable semantic worksheet identity.
pub(crate) fn worksheet_by_name(
    store: &SqliteStore,
    entities: &[ProjectEntityRecord],
    snapshot: scriberig_core::SnapshotId,
    name: &str,
) -> EntityId {
    entities
        .iter()
        .filter(|entity| entity.kind == "spreadsheet.worksheet")
        .find_map(|entity| {
            let revision = store.resolve(snapshot, entity.entity).ok().flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let metadata: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (metadata["name"] == name).then_some(entity.entity)
        })
        .expect("worksheet exists in exact snapshot")
}

/// Finds a semantic paragraph by its authored text at one exact snapshot.
///
/// - `store`: Persisted project store.
/// - `entities`: Project ownership records.
/// - `snapshot`: Exact immutable snapshot to inspect.
/// - `text`: Exact concatenated paragraph text to find.
///
/// # Returns
/// Stable semantic paragraph identity.
pub(crate) fn paragraph_by_text(
    store: &SqliteStore,
    entities: &[ProjectEntityRecord],
    snapshot: scriberig_core::SnapshotId,
    text: &str,
) -> EntityId {
    entities
        .iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store.resolve(snapshot, entity.entity).ok().flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let paragraph: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            let authored = paragraph
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>();
            (authored == text).then_some(paragraph.id)
        })
        .expect("paragraph text exists in exact snapshot")
}

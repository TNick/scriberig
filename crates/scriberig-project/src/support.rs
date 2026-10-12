//! Implements project filesystem scanning and domain-specific extraction.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, ProjectId, RootName};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::{ProjectError, invalid_input, validation_error};

#[path = "git.rs"]
mod git;
pub(crate) use git::git_tracked_paths;
#[path = "scan.rs"]
mod scan;
pub(crate) use scan::scan_directory;
/// Imports one file and appends its revisions and snapshot changes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted scan file to import.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Entity ownership records created for the file.
///
/// # Errors
/// Returns validation or canonical revision errors.
pub(crate) fn import_file(
    project: ProjectId,
    file: &ProjectFileRecord,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
) -> Result<Vec<ProjectEntityRecord>, ProjectError> {
    let source = source_entity(project, &file.relative_path);
    let mut ownership = vec![entity_record(project, file, source, "source")];
    let source_revision = revision("project.source", file.bytes.clone())?;
    changes.push(SnapshotChange::Set(source, source_revision.id()));
    revisions.push(source_revision);
    match file.adapter.as_str() {
        "markdown" => import_markdown(project, file, source, revisions, changes, &mut ownership)?,
        "rust" => import_rust(project, file, source, revisions, changes, &mut ownership)?,
        "pdf" => import_pdf(project, file, source, revisions, changes, &mut ownership)?,
        _ => {}
    }
    Ok(ownership)
}

/// Extracts semantic Markdown block entities.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Markdown source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
///
/// # Returns
/// Unit after Markdown children are added.
///
/// # Errors
/// Returns invalid source encoding or revision errors.
fn import_markdown(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let text = std::str::from_utf8(&file.bytes)
        .map_err(|error| validation_error(&file.relative_path, &error.to_string()))?;
    let Ok(parsed) = sr_markdown::parse(text) else {
        return Ok(());
    };
    for (index, block) in parsed.document.blocks.iter().enumerate() {
        add_child(
            project,
            file,
            source,
            "markdown.block",
            index,
            format!("{block:?}").into_bytes(),
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}
/// Extracts semantic Rust item entities.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Rust source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
///
/// # Returns
/// Unit after Rust children are added.
///
/// # Errors
/// Returns invalid source encoding, parse, or revision errors.
fn import_rust(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let text = std::str::from_utf8(&file.bytes)
        .map_err(|error| validation_error(&file.relative_path, &error.to_string()))?;
    let parsed = scriberig_domain_rust::parse(text)
        .map_err(|error| validation_error(&file.relative_path, &error.to_string()))?;
    for (index, item) in parsed.items.iter().enumerate() {
        add_child(
            project,
            file,
            source,
            "rust.item",
            index,
            format!("{item:?}").into_bytes(),
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}
/// Extracts semantic PDF page entities.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: PDF source file.
/// - `source`: Source artifact entity.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination entity ownership records.
///
/// # Returns
/// Unit after PDF children are added.
///
/// # Errors
/// Returns PDF validation or revision errors.
fn import_pdf(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let source_artifact = scriberig_domain_pdf::import_source(file.bytes.clone())
        .map_err(|error| validation_error(&file.relative_path, &error.to_string()))?;
    let document = scriberig_domain_pdf::extract(
        &scriberig_domain_pdf::PdfDocument::from_source(source_artifact),
        "scriberig-project",
        "1",
    );
    for (index, page) in document.pages.iter().enumerate() {
        add_child(
            project,
            file,
            source,
            "pdf.page",
            index,
            format!("{page:?}").into_bytes(),
            revisions,
            changes,
            ownership,
        )?;
    }
    Ok(())
}

/// Adds one deterministic child revision and ownership record.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Source file.
/// - `source`: Parent source entity.
/// - `kind`: Child entity kind.
/// - `index`: Child ordinal.
/// - `payload`: Canonical child payload.
/// - `revisions`: Destination revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination ownership records.
///
/// # Returns
/// Unit after the child is appended.
///
/// # Errors
/// Returns a canonical revision error.
#[allow(clippy::too_many_arguments)]
fn add_child(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    kind: &str,
    index: usize,
    payload: Vec<u8>,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let entity = child_entity(source, kind, index);
    let revision = revision(kind, payload)?;
    changes.push(SnapshotChange::Set(entity, revision.id()));
    revisions.push(revision);
    ownership.push(entity_record(project, file, entity, kind));
    Ok(())
}

/// Creates a typed immutable revision from a canonical payload.
///
/// # Parameters
/// - `kind`: Canonical object type.
/// - `payload`: Canonical payload bytes.
///
/// # Returns
/// The content-addressed revision.
///
/// # Errors
/// Returns an invalid object type or revision error.
fn revision(kind: &str, payload: Vec<u8>) -> Result<Revision, ProjectError> {
    Revision::new(
        EncodingVersion::V1,
        ObjectType::new(kind).map_err(|error| invalid_input(error.to_string()))?,
        payload,
    )
    .map_err(|error| invalid_input(format!("revision creation failed: {error:?}")))
}

/// Creates a project entity ownership record.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Source file owning the entity.
/// - `entity`: Entity identity.
/// - `kind`: Entity kind.
///
/// # Returns
/// An ownership record.
fn entity_record(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
) -> ProjectEntityRecord {
    ProjectEntityRecord {
        project_id: project,
        relative_path: file.relative_path.clone(),
        entity,
        kind: kind.to_owned(),
    }
}

/// Derives a stable source entity identity.
///
/// # Parameters
/// - `project`: Stable project identity.
/// - `path`: Project-relative path.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Stable source entity identity.
pub(crate) fn source_entity(project: ProjectId, path: &str) -> EntityId {
    let mut bytes = Vec::with_capacity(16 + path.len());
    bytes.extend_from_slice(project.as_bytes());
    bytes.extend_from_slice(path.as_bytes());
    let digest = blake3::hash(&bytes);
    let mut identity = [0; 16];
    identity.copy_from_slice(&digest.as_bytes()[..16]);
    EntityId::from_bytes(identity)
}

/// Derives a stable child entity under one source artifact.
///
/// # Parameters
/// - `source`: Parent source entity.
/// - `kind`: Child entity kind.
/// - `index`: Child ordinal.
///
/// # Returns
/// Stable child entity identity.
fn child_entity(source: EntityId, kind: &str, index: usize) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.as_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(&index.to_be_bytes());
    let mut identity = [0; 16];
    identity.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(identity)
}

/// Creates the named root assigned to one project.
///
/// # Parameters
/// - `project`: Stable project identity.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Project-specific named root.
///
/// # Errors
/// Returns an invalid root-name error.
pub(crate) fn project_root(project: ProjectId) -> Result<RootName, ProjectError> {
    RootName::new(format!("project_{project}")).map_err(|error| invalid_input(error.to_string()))
}

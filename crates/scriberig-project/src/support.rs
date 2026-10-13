//! Implements project filesystem scanning and domain-specific extraction.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::{adapter_diagnostics_error, invalid_input};

#[path = "git.rs"]
mod git;
pub(crate) use git::{git_candidate_paths, git_tracked_paths};
#[path = "scan.rs"]
mod scan;
pub(crate) use scan::{adapter_for, scan_directory};
#[path = "adapters.rs"]
mod adapters;
#[path = "support_identity.rs"]
mod support_identity;
use adapters::{
    import_binary, import_c_family, import_data, import_ecma, import_html, import_python,
    import_text,
};
pub(crate) use support_identity::{project_root, source_entity};
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
    let mut ownership = vec![entity_record(project, file, source, "source", None)];
    let source_revision = revision("project.source", file.bytes.clone())?;
    changes.push(SnapshotChange::Set(source, source_revision.id()));
    revisions.push(source_revision);
    match file.adapter.as_str() {
        "markdown" => import_markdown(project, file, source, revisions, changes, &mut ownership)?,
        "rust" => import_rust(project, file, source, revisions, changes, &mut ownership)?,
        "pdf" => import_pdf(project, file, source, revisions, changes, &mut ownership)?,
        "html" => import_html(project, file, source, revisions, changes, &mut ownership)?,
        "c" | "cpp" => {
            import_c_family(project, file, source, revisions, changes, &mut ownership)?;
        }
        "javascript" | "typescript" => {
            import_ecma(project, file, source, revisions, changes, &mut ownership)?;
        }
        "python" => {
            import_python(project, file, source, revisions, changes, &mut ownership)?;
        }
        "json" | "jsonc" | "yaml" | "toml" => {
            import_data(project, file, source, revisions, changes, &mut ownership)?;
        }
        "text" => {
            import_text(project, file, source, revisions, changes, &mut ownership)?;
        }
        "binary" => {
            import_binary(project, file, source, revisions, changes, &mut ownership)?;
        }
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
    let text = std::str::from_utf8(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "decode", "decode_failed", &[error.to_string()], true)
    })?;
    let parsed = sr_markdown::parse(text).map_err(|error| {
        adapter_diagnostics_error(file, "parse", &error.code, &[error.message], true)
    })?;
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
    let text = std::str::from_utf8(&file.bytes).map_err(|error| {
        adapter_diagnostics_error(file, "decode", "decode_failed", &[error.to_string()], true)
    })?;
    let parsed = scriberig_domain_rust::parse(text).map_err(|error| {
        adapter_diagnostics_error(file, "parse", "parse_failed", &[error.to_string()], true)
    })?;
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
    for symbol in scriberig_domain_rust::named_symbols(text).map_err(|error| {
        adapter_diagnostics_error(file, "semantic", "symbol_extraction_failed", &[error], true)
    })? {
        add_named_child(
            project,
            file,
            source,
            &symbol.kind,
            &symbol.name,
            &symbol.key,
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
    let source_artifact =
        scriberig_domain_pdf::import_source(file.bytes.clone()).map_err(|error| {
            adapter_diagnostics_error(
                file,
                "extract",
                "extract_failed",
                &[error.to_string()],
                true,
            )
        })?;
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
/// - `super`: Parent support module visibility marker.
///
/// # Returns
/// Unit after the child is appended.
///
/// # Errors
/// Returns a canonical revision error.
#[allow(clippy::too_many_arguments)]
pub(super) fn add_child(
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
    ownership.push(entity_record(project, file, entity, kind, None));
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
/// - `name`: Optional human-readable name.
///
/// # Returns
/// An ownership record.
fn entity_record(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
    name: Option<String>,
) -> ProjectEntityRecord {
    ProjectEntityRecord {
        project_id: project,
        relative_path: file.relative_path.clone(),
        entity,
        kind: kind.to_owned(),
        name,
    }
}

/// Adds one named Rust symbol to the current project snapshot.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Source file record.
/// - `source`: Source entity identity.
/// - `kind`: Semantic kind label.
/// - `name`: Human-readable symbol name.
/// - `key`: Stable semantic key.
/// - `revisions`: Output revisions.
/// - `changes`: Output snapshot changes.
/// - `ownership`: Output ownership records.
///
/// # Returns
/// Unit after appending the symbol.
///
/// # Errors
/// Returns an error when the revision cannot be created.
#[allow(clippy::too_many_arguments)]
fn add_named_child(
    project: ProjectId,
    file: &ProjectFileRecord,
    source: EntityId,
    kind: &str,
    name: &str,
    key: &str,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let entity = named_entity(source, kind, key);
    let revision = revision(kind, name.as_bytes().to_vec())?;
    changes.push(SnapshotChange::Set(entity, revision.id()));
    revisions.push(revision);
    ownership.push(entity_record(
        project,
        file,
        entity,
        kind,
        Some(name.to_owned()),
    ));
    Ok(())
}

/// Derives a stable named-symbol identity from source and semantic key.
///
/// # Parameters
/// - `source`: Source entity identity.
/// - `kind`: Semantic kind label.
/// - `key`: Stable semantic key.
///
/// # Returns
/// Stable named-symbol entity identity.
fn named_entity(source: EntityId, kind: &str, key: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.as_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(key.as_bytes());
    let mut identity = [0; 16];
    identity.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
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

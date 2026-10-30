//! Creates canonical revisions and ownership records for imported entities.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, ProjectId};
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::invalid_input;

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
/// Unit after the child is appended.
///
/// Returns a canonical revision error.
#[allow(clippy::too_many_arguments)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
    add_entity_with_id(
        project, file, entity, kind, payload, revisions, changes, ownership,
    )
}

/// Adds an explicitly assigned semantic entity to the current project snapshot.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted source file owning the semantic entity.
/// - `entity`: Stable semantic identity assigned by the domain adapter.
/// - `kind`: Canonical semantic object type.
/// - `payload`: Canonical entity payload bytes.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after the entity revision and ownership record are appended.
///
/// # Errors
/// Returns a canonical revision or object-type error.
#[allow(clippy::too_many_arguments)]
pub(crate) fn add_entity_with_id(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
    payload: Vec<u8>,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    add_entity_with_name(
        project, file, entity, kind, None, payload, revisions, changes, ownership,
    )
}

/// Adds a named semantic entity with an explicit stable identity.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Source file owning the entity.
/// - `entity`: Stable semantic identity.
/// - `kind`: Canonical semantic object type.
/// - `name`: Human-readable semantic name.
/// - `payload`: Canonical entity payload bytes.
/// - `revisions`: Destination immutable semantic revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after revision and ownership are recorded.
///
/// # Errors
/// Returns an error when the canonical revision cannot be created.
#[allow(clippy::too_many_arguments)]
pub(crate) fn add_named_entity_with_id(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
    name: &str,
    payload: Vec<u8>,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    add_entity_with_name(
        project,
        file,
        entity,
        kind,
        Some(name.to_owned()),
        payload,
        revisions,
        changes,
        ownership,
    )
}

/// Appends a revision and optional display name to the import outputs.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Source file owning the entity.
/// - `entity`: Stable semantic identity.
/// - `kind`: Canonical semantic object type.
/// - `name`: Optional human-readable entity name.
/// - `payload`: Canonical entity payload bytes.
/// - `revisions`: Destination immutable semantic revisions.
/// - `changes`: Destination snapshot changes.
/// - `ownership`: Destination project ownership records.
///
/// # Returns
/// Unit after revision and ownership are recorded.
///
/// # Errors
/// Returns an error when the canonical revision cannot be created.
#[allow(clippy::too_many_arguments)]
fn add_entity_with_name(
    project: ProjectId,
    file: &ProjectFileRecord,
    entity: EntityId,
    kind: &str,
    name: Option<String>,
    payload: Vec<u8>,
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    ownership: &mut Vec<ProjectEntityRecord>,
) -> Result<(), ProjectError> {
    let revision = revision(kind, payload)?;
    changes.push(SnapshotChange::Set(entity, revision.id()));
    revisions.push(revision);
    ownership.push(entity_record(project, file, entity, kind, name));
    Ok(())
}

/// Creates a typed immutable revision from a canonical payload.
///
/// # Parameters
/// - `kind`: Canonical object type.
/// - `payload`: Canonical payload bytes.
///
/// The content-addressed revision.
///
/// Returns an invalid object type or revision error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn revision(kind: &str, payload: Vec<u8>) -> Result<Revision, ProjectError> {
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
/// An ownership record.
/// # Returns
/// The operation result.
pub(super) fn entity_record(
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
/// Unit after appending the symbol.
///
/// Returns an error when the revision cannot be created.
#[allow(clippy::too_many_arguments)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn add_named_child(
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
/// Stable named-symbol entity identity.
/// # Returns
/// The operation result.
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
/// Stable child entity identity.
/// # Returns
/// The operation result.
fn child_entity(source: EntityId, kind: &str, index: usize) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.as_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(&index.to_be_bytes());
    let mut identity = [0; 16];
    identity.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(identity)
}

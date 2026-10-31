//! Implements project filesystem scanning and domain-specific extraction.

use std::collections::HashMap;

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::CadExternalProvider;
use scriberig_store::{Revision, SnapshotChange};
use scriberig_store_sqlite::{ProjectEntityRecord, ProjectFileRecord};

use crate::ProjectError;
use crate::errors::adapter_diagnostics_error;

#[path = "git.rs"]
mod git;
pub(crate) use git::{git_candidate_paths, git_tracked_paths};
#[path = "scan.rs"]
mod scan;
pub(crate) use scan::{adapter_for, logical_grouping_key, scan_directory};
#[path = "adapters.rs"]
pub(crate) mod adapters;
#[path = "support_identity.rs"]
mod support_identity;
use adapters::{
    import_binary, import_c_family, import_data, import_docx, import_dxf, import_ecma,
    import_geopackage, import_html, import_python, import_qgis, import_shapefile, import_text,
    import_xlsx,
};
pub(crate) use support_identity::project_root;
/// Imports one file and appends its revisions and snapshot changes.
///
/// # Parameters
/// - `project`: Owning project identity.
/// - `file`: Persisted scan file to import.
/// - `manifest`: Exact persisted scan used to resolve project-relative references.
/// - `revisions`: Destination immutable revisions.
/// - `changes`: Destination snapshot changes.
/// - `previous_cad_entities`: Prior DXF semantic entities with detached IDs by source path.
/// - `dwg_provider`: Optional host-injected DWG semantic reader.
/// - `crate`: Internal visibility marker.
///
/// Entity ownership records created for the file.
///
/// Returns validation or canonical revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn import_file(
    project: ProjectId,
    file: &ProjectFileRecord,
    manifest: &[ProjectFileRecord],
    revisions: &mut Vec<Revision>,
    changes: &mut Vec<SnapshotChange>,
    previous_cad_entities: &HashMap<String, crate::import_service::PreviousCadDrawing>,
    dwg_provider: Option<&dyn CadExternalProvider>,
) -> Result<Vec<ProjectEntityRecord>, ProjectError> {
    let source = file.entity;
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
        "xlsx" => {
            import_xlsx(project, file, source, revisions, changes, &mut ownership)?;
        }
        "dxf" => import_dxf(
            project,
            file,
            source,
            revisions,
            changes,
            &mut ownership,
            previous_cad_entities
                .get(&file.relative_path)
                .map_or(&[], |previous| previous.entities.as_slice()),
        )?,
        "dwg" => {
            if let Some(provider) = dwg_provider {
                adapters::import_dwg(
                    project,
                    file,
                    source,
                    provider,
                    previous_cad_entities.get(&file.relative_path),
                    revisions,
                    changes,
                    &mut ownership,
                )?;
            }
        }
        "docx" => {
            import_docx(project, file, source, revisions, changes, &mut ownership)?;
        }
        "qgis" => {
            import_qgis(
                project,
                file,
                manifest,
                source,
                revisions,
                changes,
                &mut ownership,
            )?;
        }
        "geopackage" => {
            import_geopackage(project, file, source, revisions, changes, &mut ownership)?;
        }
        "shapefile" => {
            import_shapefile(
                project,
                file,
                manifest,
                source,
                revisions,
                changes,
                &mut ownership,
            )?;
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
/// Unit after Markdown children are added.
///
/// Returns invalid source encoding or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// Unit after Rust children are added.
///
/// Returns invalid source encoding, parse, or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// Unit after PDF children are added.
///
/// Returns PDF validation or revision errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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

#[path = "support/entities.rs"]
mod entities;
pub(crate) use entities::revision;
use entities::{add_child, add_named_child, entity_record};
pub(super) use entities::{add_entity_with_id, add_named_entity_with_id};

//! Resolves adapter-owned preservation fidelity from exact persisted source evidence.

use scriberig_domain_cad::{CadFormat, CadProviderRegistry};
use scriberig_store_sqlite::ProjectFileRecord;

use super::{
    ArtifactCapability, ArtifactFidelity, ArtifactKind, ArtifactManifest, OpaquePreservation,
};

/// Resolves format-specific fidelity from the exact persisted source records.
///
/// XLSX fidelity depends on the retained package's edit-capability check. A
/// digest mismatch remains read-only because the bytes cannot support the scan
/// evidence used to build the manifest.
///
/// # Parameters
/// - `manifests`: Logical artifact summaries derived from the persisted scan.
/// - `sources`: Persisted source records associated with that scan.
/// - `providers`: Configured external CAD provider registry.
///
/// # Returns
/// Unit after matching adapter-specific capability data has been applied.
pub(crate) fn resolve_adapter_fidelity(
    manifests: &mut [ArtifactManifest],
    sources: &[ProjectFileRecord],
    providers: &CadProviderRegistry,
) {
    for manifest in manifests.iter_mut() {
        if manifest.kind == ArtifactKind::Shapefile {
            manifest.fidelity = shapefile_fidelity(manifest, sources);
            continue;
        }
        let adapter = manifest.source.adapter.as_str();
        if adapter == "dwg" {
            let Some(member) = manifest.members.first() else {
                continue;
            };
            let Some(path) = member.project_path.as_deref() else {
                continue;
            };
            let Some(source) = sources.iter().find(|source| source.relative_path == path) else {
                continue;
            };
            manifest.fidelity = dwg_fidelity(member.digest, source, providers);
            continue;
        }
        if !matches!(adapter, "xlsx" | "docx" | "geopackage" | "dxf") {
            continue;
        }
        let Some(member) = manifest.members.first() else {
            continue;
        };
        let Some(path) = member.project_path.as_deref() else {
            continue;
        };
        let Some(source) = sources.iter().find(|source| source.relative_path == path) else {
            continue;
        };
        manifest.fidelity = match adapter {
            "xlsx" => xlsx_fidelity(member.digest, source),
            "docx" => docx_fidelity(member.digest, source),
            "geopackage" => geopackage_fidelity(member.digest, source),
            "dxf" => dxf_fidelity(member.digest, source),
            _ => continue,
        };
    }
}

/// Evaluates optional DWG provider fidelity against exact persisted bytes.
///
/// # Parameters
/// - `scan_digest`: Digest recorded in the persisted artifact manifest.
/// - `source`: Exact DWG bytes captured by project scanning.
/// - `providers`: Explicitly injected host provider registry.
///
/// # Returns
/// Provider-derived fidelity only when the source digest matches and the
/// configured provider can reopen the exact source.
fn dwg_fidelity(
    scan_digest: [u8; 32],
    source: &ProjectFileRecord,
    providers: &CadProviderRegistry,
) -> ArtifactFidelity {
    if source.digest != scan_digest || blake3::hash(&source.bytes).as_bytes() != &scan_digest {
        return ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("DWG bytes do not match the persisted source digest".into()),
        };
    }
    let Some(provider) = providers.provider_for(&CadFormat::Dwg) else {
        let diagnostic = match providers.status(&CadFormat::Dwg) {
            scriberig_domain_cad::CadProviderStatus::Failed { .. } => {
                "DWG provider initialization failed; exact source bytes remain read-only"
            }
            _ => "No optional DWG provider is configured; exact source bytes remain read-only",
        };
        return ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(diagnostic.to_owned()),
        };
    };
    let descriptor = provider.descriptor();
    let Some(capability) = descriptor
        .formats
        .iter()
        .find(|capability| capability.format == CadFormat::Dwg)
    else {
        return ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("configured DWG provider does not advertise DWG".into()),
        };
    };
    if !capability.read {
        return crate::cad_provider_artifact_fidelity(capability);
    }
    if let Err(error) = provider.import(&CadFormat::Dwg, &source.bytes) {
        return ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(format!(
                "DWG provider could not reopen source: {:?}",
                error.kind
            )),
        };
    }
    crate::cad_provider_artifact_fidelity(capability)
}

/// Evaluates DXF editability against the exact persisted source bytes.
///
/// # Parameters
/// - `scan_digest`: Digest recorded in the persisted artifact manifest.
/// - `source`: Persisted DXF source record retained by the project scan.
///
/// # Returns
/// Lossless editing capability only when every source structure maps to the
/// supported semantic subset; incomplete and stale evidence stays read-only.
fn dxf_fidelity(scan_digest: [u8; 32], source: &ProjectFileRecord) -> ArtifactFidelity {
    if source.digest != scan_digest || blake3::hash(&source.bytes).as_bytes() != &scan_digest {
        return ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("DXF bytes do not match the persisted source digest".into()),
        };
    }
    match scriberig_domain_dxf::import_dxf(&source.bytes, source.entity) {
        Ok(imported) if imported.complete => ArtifactFidelity {
            capability: ArtifactCapability::LosslessEditable,
            opaque_preservation: OpaquePreservation::NotPresent,
            diagnostic: None,
        },
        Ok(imported) if scriberig_domain_dxf::supports_opaque_preserving_edits(&source.bytes) => {
            ArtifactFidelity {
                capability: ArtifactCapability::EditableWithOpaquePreservation,
                opaque_preservation: OpaquePreservation::Preserved,
                diagnostic: Some(format!(
                    "unsupported DXF content is retained exactly; edits are limited to verified existing fields on imported entities: {}",
                    imported.diagnostics.join("; ")
                )),
            }
        }
        Ok(imported) => ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(imported.diagnostics.join("; ")),
        },
        Err(error) => ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(format!("DXF source could not be reopened: {error}")),
        },
    }
}

/// Evaluates read-only Shapefile semantics against every exact compound member.
///
/// # Parameters
/// - `manifest`: Logical Shapefile group and captured member digests.
/// - `sources`: Persisted project members from the same scan.
///
/// # Returns
/// Read-only derived fidelity for valid complete sets, otherwise inspect-only.
fn shapefile_fidelity(
    manifest: &ArtifactManifest,
    sources: &[ProjectFileRecord],
) -> ArtifactFidelity {
    let mut present = std::collections::BTreeSet::new();
    let mut matched = Vec::new();
    for member in &manifest.members {
        let Some(path) = member.project_path.as_deref() else {
            return shapefile_inspect_only("Shapefile member path evidence is incomplete");
        };
        let Some(source) = sources.iter().find(|source| source.relative_path == path) else {
            return shapefile_inspect_only(
                "Shapefile source member is absent from the captured scan",
            );
        };
        if source.digest != member.digest {
            return shapefile_inspect_only("Shapefile member digest does not match scan evidence");
        }
        if let Some((_, extension)) = path.rsplit_once('.') {
            present.insert(extension.to_ascii_lowercase());
        }
        matched.push(source);
    }
    let missing = ["shp", "shx", "dbf"]
        .into_iter()
        .filter(|extension| !present.contains(*extension))
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return shapefile_inspect_only(&format!(
            "Shapefile is missing mandatory sidecars: {}",
            missing.join(", ")
        ));
    }
    let Some(shape) = matched
        .iter()
        .find(|source| source.relative_path.to_ascii_lowercase().ends_with(".shp"))
    else {
        return shapefile_inspect_only("Shapefile geometry member is absent");
    };
    let sidecar = |extension: &str| {
        let expected = replace_extension(&shape.relative_path, extension);
        matched
            .iter()
            .find(|source| source.relative_path.eq_ignore_ascii_case(&expected))
            .map(|source| source.bytes.as_slice())
    };
    let dataset = crate::support::adapters::shapefile::dataset_semantic_id(shape.entity);
    match scriberig_domain_geo::ShapefileReader::open(
        &shape.bytes,
        sidecar("dbf"),
        sidecar("prj"),
        dataset,
        std::path::Path::new(&shape.relative_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("shapefile"),
    ) {
        Ok(_) => ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(
                "Shapefile semantics are readable; compound materialization remains inspect-only until atomic sidecar writes are verified".into(),
            ),
        },
        Err(_) => shapefile_inspect_only("Shapefile sidecars could not be reopened together"),
    }
}

/// Creates explicit inspect-only Shapefile fidelity with a safe diagnostic.
///
/// # Parameters
/// - `diagnostic`: Safe reason semantic source inspection is restricted.
///
/// # Returns
/// Fidelity that permits inspection but no semantic write claim.
fn shapefile_inspect_only(diagnostic: &str) -> ArtifactFidelity {
    ArtifactFidelity {
        capability: ArtifactCapability::InspectOnly,
        opaque_preservation: OpaquePreservation::Unproven,
        diagnostic: Some(diagnostic.to_owned()),
    }
}

/// Replaces a relative filename extension while preserving its directory.
///
/// # Parameters
/// - `path`: Project-relative member path.
/// - `extension`: Lowercase suffix without a leading dot.
///
/// # Returns
/// Same-stem project-relative path with the requested suffix.
fn replace_extension(path: &str, extension: &str) -> String {
    let mut path = std::path::PathBuf::from(path);
    path.set_extension(extension);
    path.to_string_lossy().replace('\\', "/")
}

/// Evaluates `GeoPackage` staged-writer capability against exact source bytes.
///
/// # Parameters
/// - `scan_digest`: Exact source digest recorded by the persisted scan.
/// - `source`: Persisted package bytes and source identity.
///
/// # Returns
/// Read-only or layer-aware editable package fidelity.
fn geopackage_fidelity(scan_digest: [u8; 32], source: &ProjectFileRecord) -> ArtifactFidelity {
    if source.digest != scan_digest {
        return ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("persisted GeoPackage digest does not match scan evidence".into()),
        };
    }
    match scriberig_domain_geo::GeoPackageReader::open(
        &source.bytes,
        scriberig_core::EntityId::new(),
    ) {
        Ok(reader) if reader.layers().iter().any(|layer| layer.editable) => ArtifactFidelity {
            capability: ArtifactCapability::LossAwareEditable,
            opaque_preservation: OpaquePreservation::Preserved,
            diagnostic: Some(
                "only layers within the staged geometry writer subset accept edits".into(),
            ),
        },
        Ok(_) => ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Preserved,
            diagnostic: Some("no GeoPackage layer supports staged geometry edits".into()),
        },
        Err(_) => ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("GeoPackage source could not be reopened".into()),
        },
    }
}

/// Evaluates DOCX import fidelity against its exact persisted package bytes.
///
/// # Parameters
/// - `scan_digest`: Exact source digest recorded by the persisted scan.
/// - `source`: Persisted package bytes and their source identity.
///
/// # Returns
/// Public fidelity classification and adapter diagnostic for the DOCX artifact.
fn docx_fidelity(scan_digest: [u8; 32], source: &ProjectFileRecord) -> ArtifactFidelity {
    if source.digest != scan_digest {
        return ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("persisted DOCX source digest does not match scan evidence".into()),
        };
    }
    match scriberig_domain_docx::import_docx(
        &source.bytes,
        scriberig_domain_docx::semantic_document_id(source.entity),
    ) {
        Ok(imported) if imported.editable => ArtifactFidelity {
            capability: ArtifactCapability::EditableWithOpaquePreservation,
            opaque_preservation: OpaquePreservation::Preserved,
            diagnostic: imported.diagnostic,
        },
        Ok(imported) => ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: imported.diagnostic,
        },
        Err(error) => ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(format!("docx_import_failed: {error}")),
        },
    }
}

/// Evaluates XLSX writer capability against its exact persisted package bytes.
///
/// # Parameters
/// - `scan_digest`: Exact source digest recorded by the persisted scan.
/// - `source`: Persisted package bytes and their source identity.
///
/// # Returns
/// Public fidelity classification for the XLSX artifact.
fn xlsx_fidelity(scan_digest: [u8; 32], source: &ProjectFileRecord) -> ArtifactFidelity {
    if source.digest != scan_digest {
        return ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some("persisted XLSX source digest does not match scan evidence".into()),
        };
    }
    match scriberig_domain_xlsx::import_xlsx(
        &source.bytes,
        scriberig_domain_xlsx::semantic_workbook_id(source.entity),
    ) {
        Ok(imported) if imported.editable => ArtifactFidelity {
            capability: ArtifactCapability::EditableWithOpaquePreservation,
            opaque_preservation: OpaquePreservation::Preserved,
            diagnostic: None,
        },
        Ok(imported) => ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: imported.diagnostic,
        },
        Err(error) => ArtifactFidelity {
            capability: ArtifactCapability::InspectOnly,
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(format!("{}: {}", error.code, error.message)),
        },
    }
}

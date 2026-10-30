//! Groups exact persisted project evidence into logical artifact manifests.

use std::collections::BTreeMap;

use scriberig_core::ProjectId;
use scriberig_domain_geo::shapefile_member_path;
use scriberig_store_sqlite::ProjectFileDescriptor;

use super::{
    ArtifactCapability, ArtifactFidelity, ArtifactGroupingDiagnostic, ArtifactId, ArtifactKind,
    ArtifactManifest, ArtifactMemberDescriptor, ArtifactMemberKey, ArtifactSourceEvidence,
    OpaquePreservation,
};

/// Groups a persisted source manifest into logical artifacts using adapter rules.
///
/// # Parameters
/// - `project`: Project that owns the persisted source records.
/// - `files`: Exact records read from the persisted project scan.
///
/// # Returns
/// Deterministically ordered logical artifact manifests.
#[must_use]
pub fn group_persisted_files(
    project: ProjectId,
    files: &[ProjectFileDescriptor],
) -> Vec<ArtifactManifest> {
    group_persisted_files_with_diagnostics(project, files).0
}

/// Groups persisted files and reports candidates whose membership is ambiguous.
///
/// Shapefile grouping is case-insensitive on common target filesystems. If two
/// differently cased stems collide, each source file remains a separate
/// artifact and a diagnostic records why compound grouping was declined.
///
/// # Parameters
/// - `project`: Project that owns the persisted source records.
/// - `files`: Exact records read from the persisted project scan.
///
/// # Returns
/// Deterministic artifact manifests and grouping diagnostics.
#[must_use]
pub fn group_persisted_files_with_diagnostics(
    project: ProjectId,
    files: &[ProjectFileDescriptor],
) -> (Vec<ArtifactManifest>, Vec<ArtifactGroupingDiagnostic>) {
    let scan_id = scan_identity(files);
    let (ambiguous_stems, diagnostics) = ambiguous_shapefile_stems(files);
    let mut groups: BTreeMap<String, Vec<&ProjectFileDescriptor>> = BTreeMap::new();
    for file in files {
        let (key, _) = grouping_key(file);
        let key = if ambiguous_stems.contains(&casefold_shapefile_stem(file)) {
            format!("file:{}", file.relative_path)
        } else {
            key
        };
        groups.entry(key).or_default().push(file);
    }
    let mut manifests = Vec::with_capacity(groups.len());
    let mut diagnostics = diagnostics;
    for (logical_key, members) in groups {
        if ambiguous_stems.contains(&casefold_shapefile_stem(members[0])) {
            manifests.push(build_single_file_manifest(
                project,
                scan_id,
                logical_key,
                members[0],
            ));
            continue;
        }
        let missing = missing_required_shapefile_sidecars(&members);
        if !missing.is_empty() {
            diagnostics.push(ArtifactGroupingDiagnostic {
                code: "shapefile-missing-required-sidecar".to_owned(),
                logical_key: logical_key.clone(),
                paths: members
                    .iter()
                    .map(|member| member.relative_path.clone())
                    .collect(),
                message: format!(
                    "Shapefile artifact is missing mandatory sidecars: {}",
                    missing.join(", ")
                ),
            });
        }
        manifests.push(build_manifest(project, scan_id, logical_key, &members));
    }
    (manifests, diagnostics)
}

/// Lists required Shapefile members absent from one persisted artifact group.
///
/// # Parameters
/// - `files`: Exact persisted members grouped under one logical dataset.
///
/// # Returns
/// Missing mandatory filename extensions in stable order.
fn missing_required_shapefile_sidecars(files: &[&ProjectFileDescriptor]) -> Vec<String> {
    let present = files
        .iter()
        .filter_map(|file| file.relative_path.rsplit_once('.').map(|(_, ext)| ext))
        .map(str::to_ascii_lowercase)
        .collect::<std::collections::BTreeSet<_>>();
    ["shp", "shx", "dbf"]
        .into_iter()
        .filter(|extension| !present.contains(*extension))
        .map(|extension| format!(".{extension}"))
        .collect()
}

/// Finds case-insensitive Shapefile stem collisions in persisted evidence.
///
/// # Parameters
/// - `files`: Persisted project scan records.
///
/// # Returns
/// Ambiguous normalized stems and diagnostics in deterministic order.
fn ambiguous_shapefile_stems(
    files: &[ProjectFileDescriptor],
) -> (
    std::collections::BTreeSet<String>,
    Vec<ArtifactGroupingDiagnostic>,
) {
    let mut stems: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for file in files {
        let Some(member) = shapefile_member_path(&file.relative_path) else {
            continue;
        };
        stems
            .entry(member.normalized_stem)
            .or_default()
            .entry(member.stem)
            .or_default()
            .push(file.relative_path.clone());
    }
    let mut ambiguous = std::collections::BTreeSet::new();
    let mut diagnostics = Vec::new();
    for (normalized, variants) in stems {
        if variants.len() < 2 {
            continue;
        }
        ambiguous.insert(normalized.clone());
        let mut paths = variants.into_values().flatten().collect::<Vec<_>>();
        paths.sort();
        diagnostics.push(ArtifactGroupingDiagnostic {
            code: "artifact-grouping-case-collision".to_owned(),
            logical_key: format!("shapefile:{normalized}"),
            paths,
            message:
                "Shapefile members have case-insensitive stem collisions; files were kept separate"
                    .to_owned(),
        });
    }
    (ambiguous, diagnostics)
}

/// Returns the normalized Shapefile grouping stem, when applicable.
///
/// # Parameters
/// - `file`: Persisted source record.
///
/// # Returns
/// The case-folded relative stem for Shapefile member extensions.
fn casefold_shapefile_stem(file: &ProjectFileDescriptor) -> String {
    shapefile_member_path(&file.relative_path)
        .map_or_else(String::new, |member| member.normalized_stem)
}

/// Builds a non-grouped artifact for one ambiguous Shapefile member.
///
/// # Parameters
/// - `project`: Project owning the source file.
/// - `scan_id`: Exact persisted scan identity.
/// - `logical_key`: File-specific fallback artifact key.
/// - `file`: Persisted source member.
///
/// # Returns
/// A single-file manifest that does not infer compound membership.
fn build_single_file_manifest(
    project: ProjectId,
    scan_id: [u8; 32],
    logical_key: String,
    file: &ProjectFileDescriptor,
) -> ArtifactManifest {
    let mut manifest = build_manifest(project, scan_id, logical_key, &[file]);
    manifest.kind = ArtifactKind::SingleFile(file.adapter.clone());
    manifest
}

/// Hashes exact persisted paths, adapters, classifications, and source digests.
///
/// # Parameters
/// - `files`: Exact records read from the persisted project scan.
///
/// # Returns
/// Stable scan-manifest digest independent of directory traversal order.
fn scan_identity(files: &[ProjectFileDescriptor]) -> [u8; 32] {
    let mut ordered = files.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-scan-v1\0");
    for file in ordered {
        for value in [
            file.relative_path.as_bytes(),
            file.adapter.as_bytes(),
            file.status.as_bytes(),
            file.grouping_key.as_deref().unwrap_or_default().as_bytes(),
            file.digest.as_slice(),
        ] {
            hasher.update(&(value.len() as u64).to_be_bytes());
            hasher.update(value);
        }
    }
    *hasher.finalize().as_bytes()
}

/// Selects a stable grouping key and artifact kind for one persisted file.
///
/// # Parameters
/// - `file`: Persisted source evidence to classify.
///
/// # Returns
/// Logical artifact key and kind.
fn grouping_key(file: &ProjectFileDescriptor) -> (String, ArtifactKind) {
    let path = file.relative_path.as_str();
    let (base, kind) = if let Some(member) = shapefile_member_path(path) {
        (member.stem, ArtifactKind::Shapefile)
    } else {
        match file.adapter.as_str() {
            "xlsx" => (path.to_owned(), ArtifactKind::Xlsx),
            "docx" => (path.to_owned(), ArtifactKind::Docx),
            _ => (
                path.to_owned(),
                ArtifactKind::SingleFile(file.adapter.clone()),
            ),
        }
    };
    let fallback = match &kind {
        ArtifactKind::Shapefile => format!("shapefile:{base}"),
        ArtifactKind::Xlsx => format!("xlsx:{base}"),
        ArtifactKind::Docx => format!("docx:{base}"),
        ArtifactKind::SingleFile(_) => format!("file:{base}"),
        ArtifactKind::Adapter(_) => unreachable!(),
    };
    (file.grouping_key.clone().unwrap_or(fallback), kind)
}

/// Builds one manifest from records sharing an adapter grouping key.
///
/// # Parameters
/// - `project`: Project owning the files.
/// - `scan_id`: Exact persisted scan identity.
/// - `logical_key`: Stable grouping key.
/// - `files`: Persisted members in the group.
///
/// # Returns
/// A deterministic exact-evidence manifest.
fn build_manifest(
    project: ProjectId,
    scan_id: [u8; 32],
    logical_key: String,
    files: &[&ProjectFileDescriptor],
) -> ArtifactManifest {
    let (_, kind) = grouping_key(files[0]);
    let missing = if kind == ArtifactKind::Shapefile {
        missing_required_shapefile_sidecars(files)
    } else {
        Vec::new()
    };
    let adapter = if kind == ArtifactKind::Shapefile {
        "shapefile".to_owned()
    } else {
        files[0].adapter.clone()
    };
    let mut members = files
        .iter()
        .map(|file| ArtifactMemberDescriptor {
            key: ArtifactMemberKey(file.relative_path.clone()),
            project_path: Some(file.relative_path.clone()),
            container_part: None,
            digest: file.digest,
        })
        .collect::<Vec<_>>();
    members.sort_by(|left, right| left.key.cmp(&right.key));
    let identity_source = files
        .iter()
        .find(|file| file.relative_path.to_ascii_lowercase().ends_with(".shp"))
        .or_else(|| {
            files
                .iter()
                .min_by(|left, right| left.relative_path.cmp(&right.relative_path))
        })
        .expect("artifact groups are non-empty")
        .entity;
    ArtifactManifest {
        id: ArtifactId::from_source_entity(identity_source),
        logical_key,
        kind,
        members,
        source: ArtifactSourceEvidence {
            project,
            scan_id,
            adapter: adapter.clone(),
            adapter_version: (adapter == scriberig_domain_xlsx::ADAPTER_ID)
                .then(|| scriberig_domain_xlsx::ADAPTER_VERSION.to_owned()),
            classification: files[0].status.clone(),
        },
        fidelity: ArtifactFidelity {
            capability: if missing.is_empty() {
                ArtifactCapability::ReadOnlyDerived
            } else {
                ArtifactCapability::InspectOnly
            },
            opaque_preservation: OpaquePreservation::Unproven,
            diagnostic: Some(if !missing.is_empty() {
                format!(
                    "Shapefile artifact lacks required sidecars: {}",
                    missing.join(", ")
                )
            } else if adapter == "dwg" {
                "No optional DWG provider is configured; exact source bytes remain read-only".into()
            } else {
                "No artifact writer has declared preservation guarantees".into()
            }),
        },
        references: Vec::new(),
    }
}

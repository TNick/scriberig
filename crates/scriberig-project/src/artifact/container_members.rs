//! Adds exact package-part digests to artifact manifests from retained source bytes.

use std::io::Read;

use scriberig_store_sqlite::ProjectFileRecord;

use super::{ArtifactKind, ArtifactManifest, ArtifactMemberDescriptor, ArtifactMemberKey};

/// Appends deterministic container-internal member descriptors to supported packages.
///
/// Package entries are hashed as they are read and never extracted to project
/// paths. Invalid, duplicate, or unsafe entries produce no partial part list;
/// format fidelity will independently report whether the source is editable.
///
/// # Parameters
/// - `manifests`: Artifact manifests built from exact persisted scan records.
/// - `sources`: Exact file bytes captured by the same persisted scan.
pub(crate) fn append_container_member_evidence(
    manifests: &mut [ArtifactManifest],
    sources: &[ProjectFileRecord],
) {
    for manifest in manifests {
        let qgz_package = manifest.source.adapter == "qgis"
            && manifest.members.iter().any(|member| {
                member.project_path.as_deref().is_some_and(|path| {
                    path.rsplit_once('.')
                        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("qgz"))
                })
            });
        if !matches!(manifest.kind, ArtifactKind::Xlsx | ArtifactKind::Docx) && !qgz_package {
            continue;
        }
        let Some(file_member) = manifest
            .members
            .iter()
            .find(|member| member.project_path.is_some() && member.container_part.is_none())
        else {
            continue;
        };
        let Some(path) = file_member.project_path.as_deref() else {
            continue;
        };
        let Some(source) = sources
            .iter()
            .find(|source| source.relative_path == path && source.digest == file_member.digest)
        else {
            continue;
        };
        let Some(parts) = package_members(&source.bytes) else {
            continue;
        };
        manifest.members.extend(parts);
        manifest.members.sort_by(|left, right| {
            left.project_path
                .is_none()
                .cmp(&right.project_path.is_none())
                .then_with(|| left.key.cmp(&right.key))
        });
    }
}

/// Reads normalized package-part names and content digests from a ZIP package.
///
/// # Parameters
/// - `bytes`: Exact persisted package bytes.
///
/// # Returns
/// Deterministically ordered part descriptors, or `None` for ambiguous or invalid ZIP data.
fn package_members(bytes: &[u8]) -> Option<Vec<ArtifactMemberDescriptor>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut names = std::collections::BTreeSet::new();
    let mut members = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut part = archive.by_index(index).ok()?;
        if part.is_dir() {
            continue;
        }
        let name = part.name().to_owned();
        if !safe_part_name(&name) || !names.insert(name.clone()) {
            return None;
        }
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let count = part.read(&mut buffer).ok()?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        members.push(ArtifactMemberDescriptor {
            key: ArtifactMemberKey(format!("container:{name}")),
            project_path: None,
            container_part: Some(name),
            digest: *hasher.finalize().as_bytes(),
        });
    }
    members.sort_by(|left, right| left.key.cmp(&right.key));
    Some(members)
}

/// Rejects package member names that cannot serve as safe logical part keys.
///
/// # Parameters
/// - `name`: Exact ZIP member name.
///
/// # Returns
/// True when the name is a relative slash-separated package path without traversal.
fn safe_part_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && name
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

//! Compares live artifact members with their persisted materialization baseline.

use std::path::{Component, Path};

use scriberig_core::EntityId;

use crate::ArtifactManifest;

/// Reports whether live project members match an artifact's persisted baseline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactSynchronizationStatus {
    /// Stable semantic artifact identity.
    pub artifact: EntityId,
    /// Digest of the exact persisted member baseline.
    pub baseline_digest: String,
    /// Digest of observed live members when every member was readable.
    pub observed_digest: Option<String>,
    /// Live synchronization outcome.
    pub status: ArtifactSynchronizationState,
}

/// States the observed relationship between an artifact and its source baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactSynchronizationState {
    /// Every live member matches its persisted digest.
    InSync,
    /// At least one live member differs from its persisted digest.
    Modified,
    /// At least one required live member is absent.
    Missing,
    /// A member could not be inspected safely or read completely.
    Unavailable,
}

/// Checks every physical member of one logical artifact against its baseline.
///
/// - `root`: Registered project root.
/// - `artifact`: Persisted artifact manifest and exact member digests.
///
/// Exact baseline and observed aggregate digests when all members are available.
/// # Returns
/// Synchronization state without exposing source content.
pub fn synchronization_for_artifact(
    root: &Path,
    artifact: &ArtifactManifest,
) -> ArtifactSynchronizationStatus {
    let mut baseline = blake3::Hasher::new();
    let mut observed = blake3::Hasher::new();
    let mut modified = false;
    let mut missing = false;
    let mut unavailable = false;
    let mut physical_count = 0usize;
    let mut observed_complete = true;
    for member in artifact
        .members
        .iter()
        .filter(|member| member.project_path.is_some())
    {
        let relative = member
            .project_path
            .as_deref()
            .expect("filtered project path");
        physical_count += 1;
        hash_member(&mut baseline, relative, &member.digest);
        match read_project_member_digest(root, relative) {
            Ok(digest) => {
                hash_member(&mut observed, relative, &digest);
                if digest != member.digest {
                    modified = true;
                }
            }
            Err(MemberReadFailure::Missing) => {
                observed_complete = false;
                missing = true;
            }
            Err(MemberReadFailure::Unavailable) => {
                observed_complete = false;
                unavailable = true;
            }
        }
    }
    if physical_count == 0 {
        observed_complete = false;
        unavailable = true;
    }
    let status = if unavailable {
        ArtifactSynchronizationState::Unavailable
    } else if missing {
        ArtifactSynchronizationState::Missing
    } else if modified {
        ArtifactSynchronizationState::Modified
    } else {
        ArtifactSynchronizationState::InSync
    };
    ArtifactSynchronizationStatus {
        artifact: artifact.id.entity(),
        baseline_digest: baseline.finalize().to_hex().to_string(),
        observed_digest: observed_complete.then(|| observed.finalize().to_hex().to_string()),
        status,
    }
}

/// Adds one path and digest pair to a deterministic artifact-level digest.
///
/// - `hasher`: Aggregate digest being built.
/// - `path`: Stable project-relative member path.
/// - `digest`: Exact member digest bytes.
fn hash_member(hasher: &mut blake3::Hasher, path: &str, digest: &[u8]) {
    hasher.update(&(path.len() as u64).to_le_bytes());
    hasher.update(path.as_bytes());
    hasher.update(digest);
}

/// Describes why a live project member could not be read for synchronization.
enum MemberReadFailure {
    /// A required member is absent from the project tree.
    Missing,
    /// A member is unsafe, unreadable, or otherwise unavailable.
    Unavailable,
}

/// Computes one member digest without following paths outside the project root.
///
/// - `root`: Registered project root.
/// - `relative`: Persisted project-relative member path.
///
/// # Returns
/// Exact BLAKE3 digest of live member bytes.
///
/// # Errors
/// Returns a failure classification when the member is absent or cannot be safely read.
fn read_project_member_digest(root: &Path, relative: &str) -> Result<[u8; 32], MemberReadFailure> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(MemberReadFailure::Unavailable);
    }
    let project_root = root
        .canonicalize()
        .map_err(|_| MemberReadFailure::Unavailable)?;
    let candidate = root.join(relative_path);
    let canonical = match candidate.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(MemberReadFailure::Missing);
        }
        Err(_) => return Err(MemberReadFailure::Unavailable),
    };
    if !canonical.starts_with(project_root) {
        return Err(MemberReadFailure::Unavailable);
    }
    crate::project_security::require_public_path(&canonical)
        .map_err(|_| MemberReadFailure::Unavailable)?;
    let mut file = std::fs::File::open(canonical).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            MemberReadFailure::Missing
        } else {
            MemberReadFailure::Unavailable
        }
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(&mut file)
        .map_err(|_| MemberReadFailure::Unavailable)?;
    Ok(*hasher.finalize().as_bytes())
}

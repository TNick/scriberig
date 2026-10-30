//! Verifies multi-member artifact writes report and recover partial failures.

#[path = "artifact_materialization/failure.rs"]
mod failure;
#[path = "artifact_materialization/multi_member.rs"]
mod multi_member;
#[path = "artifact_materialization/opaque.rs"]
mod opaque;
#[path = "artifact_materialization/single_member.rs"]
mod single_member;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use scriberig_core::EntityId;
use sr_projection_fs::{
    ArtifactMaterializationMember, ArtifactMaterializationPlan, ArtifactMaterializationState,
    ArtifactMemberReceipt, FileMetadata, FileSystem, IoError, digest_bytes, materialize_artifact,
    materialize_artifact_validated, materialize_artifact_validated_with_progress,
};

/// Supplies deterministic in-memory files and an injectable write failure.
#[derive(Default)]
struct FakeFileSystem {
    files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    fail_write: Mutex<Option<usize>>,
    writes: Mutex<usize>,
}

impl FakeFileSystem {
    /// Seeds one file with exact bytes.
    ///
    /// # Parameters
    /// - `path`: Fake filesystem path to seed.
    /// - `bytes`: Exact file contents.
    fn seed(&self, path: impl Into<PathBuf>, bytes: &[u8]) {
        self.files
            .lock()
            .expect("files")
            .insert(path.into(), bytes.to_vec());
    }

    /// Fails the selected one-based write attempt.
    ///
    /// # Parameters
    /// - `attempt`: One-based atomic write number to fail.
    fn fail_on(&self, attempt: usize) {
        *self.fail_write.lock().expect("failure") = Some(attempt);
    }
}

impl FileSystem for FakeFileSystem {
    /// Reads exact bytes from the fake filesystem.
    ///
    /// # Parameters
    /// - `path`: File path to read.
    ///
    /// # Returns
    /// Exact stored bytes.
    ///
    /// # Errors
    /// Returns a not-found IO error when the path is absent.
    fn read(&self, path: &Path) -> Result<Vec<u8>, IoError> {
        self.files
            .lock()
            .expect("files")
            .get(path)
            .cloned()
            .ok_or_else(|| IoError::Read(std::io::ErrorKind::NotFound.into()))
    }

    /// Looks up fake file existence and byte length.
    ///
    /// # Parameters
    /// - `path`: File path to inspect.
    ///
    /// # Returns
    /// File metadata when the path exists.
    ///
    /// # Errors
    /// This fake metadata lookup does not fail.
    fn metadata(&self, path: &Path) -> Result<Option<FileMetadata>, IoError> {
        Ok(self
            .files
            .lock()
            .expect("files")
            .get(path)
            .map(|bytes| FileMetadata {
                exists: true,
                length: bytes.len() as u64,
            }))
    }

    /// Atomically stores fake bytes unless the configured write fails.
    ///
    /// # Parameters
    /// - `path`: Target path to replace.
    /// - `bytes`: Exact new file contents.
    ///
    /// # Returns
    /// Unit after replacement.
    ///
    /// # Errors
    /// Returns a write error on the configured attempt.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), IoError> {
        let mut writes = self.writes.lock().expect("writes");
        *writes += 1;
        if *self.fail_write.lock().expect("failure") == Some(*writes) {
            return Err(IoError::Write(std::io::ErrorKind::Other.into()));
        }
        self.files
            .lock()
            .expect("files")
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    /// Accepts directory preparation for the in-memory filesystem.
    ///
    /// # Parameters
    /// - `_path`: Directory path requested by the caller.
    ///
    /// # Returns
    /// Unit after accepting directory preparation.
    ///
    /// # Errors
    /// This fake directory preparation does not fail.
    fn create_dir_all(&self, _path: &Path) -> Result<(), IoError> {
        Ok(())
    }

    /// Removes a fake file.
    ///
    /// # Parameters
    /// - `path`: File path to remove.
    ///
    /// # Returns
    /// Unit after removing the path when present.
    ///
    /// # Errors
    /// This fake removal does not fail.
    fn remove_file(&self, path: &Path) -> Result<(), IoError> {
        self.files.lock().expect("files").remove(path);
        Ok(())
    }
}

/// Creates a three-member Shapefile plan from exact baseline and output bytes.
///
/// # Returns
/// A deterministic Shapefile replacement plan with all required sidecars.
fn plan() -> ArtifactMaterializationPlan {
    ArtifactMaterializationPlan {
        artifact: EntityId::from_bytes([5; 16]),
        root: PathBuf::from("/project"),
        members: vec![
            ArtifactMaterializationMember {
                key: "geometry".into(),
                relative_path: PathBuf::from("roads.shp"),
                expected_digest: Some(digest_bytes(b"old-shape")),
                required: true,
                bytes: Some(b"new-shape".to_vec()),
            },
            ArtifactMaterializationMember {
                key: "index".into(),
                relative_path: PathBuf::from("roads.shx"),
                expected_digest: Some(digest_bytes(b"old-index")),
                required: true,
                bytes: Some(b"new-index".to_vec()),
            },
            ArtifactMaterializationMember {
                key: "attributes".into(),
                relative_path: PathBuf::from("roads.dbf"),
                expected_digest: Some(digest_bytes(b"old-table")),
                required: true,
                bytes: Some(b"new-table".to_vec()),
            },
        ],
        opaque_content: Vec::new(),
    }
}

/// Deletes an exact baseline member and records an absent output digest.
#[test]
fn member_deletion_is_materialized_and_receipted() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/obsolete.part", b"old-part");
    let plan = ArtifactMaterializationPlan {
        artifact: EntityId::from_bytes([14; 16]),
        root: PathBuf::from("/project"),
        members: vec![ArtifactMaterializationMember {
            key: "obsolete-part".into(),
            relative_path: PathBuf::from("obsolete.part"),
            expected_digest: Some(digest_bytes(b"old-part")),
            required: true,
            bytes: None,
        }],
        opaque_content: Vec::new(),
    };

    let receipt = materialize_artifact(&filesystem, &plan, None).expect("delete member");

    assert_eq!(receipt.state, ArtifactMaterializationState::Complete);
    assert!(receipt.members[0].committed);
    assert_eq!(receipt.members[0].output_digest, None);
    assert!(
        filesystem
            .metadata(Path::new("/project/obsolete.part"))
            .expect("metadata")
            .is_none()
    );
}

/// Records an external modification before replacing any artifact member.
#[test]
fn concurrent_change_blocks_materialization_before_first_write() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"changed externally");
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let receipt = materialize_artifact(&filesystem, &plan(), None).expect("conflict receipt");
    assert_eq!(
        receipt.state,
        ArtifactMaterializationState::BaselineConflict
    );
    assert!(receipt.members[0].failure.is_some());
    assert_eq!(
        receipt.members[0].observed_digest,
        Some(digest_bytes(b"changed externally")),
        "conflict receipt retains the exact competing member digest"
    );
    assert_eq!(*filesystem.writes.lock().expect("writes"), 0);
}

/// Rejects invalid staged output without touching any live artifact member.
#[test]
fn validation_failure_prevents_all_member_replacements() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"old-shape");
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let receipt = materialize_artifact_validated(&filesystem, &plan(), None, |staged| {
        assert_eq!(staged.members.len(), 3);
        assert_eq!(staged.members[0].bytes.as_deref(), Some(&b"new-shape"[..]));
        Err("candidate topology validation failed".to_owned())
    })
    .expect("validation receipt");
    assert_eq!(
        receipt.state,
        ArtifactMaterializationState::ValidationFailed
    );
    assert!(receipt.members.iter().all(|member| !member.committed));
    assert_eq!(*filesystem.writes.lock().expect("writes"), 0);
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shp"))
            .expect("unchanged shape"),
        b"old-shape"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.dbf"))
            .expect("unchanged table"),
        b"old-table"
    );
}

/// Rejects distinct artifact members that would overwrite the same target path.
#[test]
fn duplicate_target_path_is_rejected_before_any_write() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"old-shape");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let mut plan = plan();
    plan.members[1].relative_path = plan.members[0].relative_path.clone();

    let result = materialize_artifact(&filesystem, &plan, None);

    assert!(matches!(result, Err(IoError::UnsafeArtifactPlan)));
    assert_eq!(*filesystem.writes.lock().expect("writes"), 0);
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shp"))
            .expect("unchanged shape"),
        b"old-shape"
    );
}

/// Rejects path aliases that normalize to the same target as another member.
#[test]
fn current_directory_path_component_is_rejected_before_writing() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"old-shape");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let mut plan = plan();
    plan.members[1].relative_path = PathBuf::from("./roads.shp");

    let result = materialize_artifact(&filesystem, &plan, None);

    assert!(matches!(result, Err(IoError::UnsafeArtifactPlan)));
    assert_eq!(*filesystem.writes.lock().expect("writes"), 0);
}

/// Keeps the member receipt shape explicit for public recovery callers.
#[test]
fn member_receipt_identifies_exact_staged_digest() {
    let output = b"generated";
    let receipt = ArtifactMemberReceipt {
        key: "part".into(),
        output_digest: Some(digest_bytes(output)),
        observed_digest: None,
        committed: false,
        failure: Some("injected".into()),
    };
    assert_eq!(receipt.output_digest, Some(digest_bytes(output)));
    assert!(!receipt.committed);
}

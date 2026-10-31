//! Verifies single-member artifacts use an exact atomic replacement.

use std::path::PathBuf;

use scriberig_core::EntityId;
use sr_projection_fs::{
    ArtifactMaterializationMember, ArtifactMaterializationPlan, ArtifactMaterializationState,
    FileSystem, digest_bytes, materialize_artifact,
};

use super::FakeFileSystem;

/// Keeps a one-file artifact on the existing per-file atomic replacement path.
#[test]
fn single_file_artifact_uses_atomic_write_and_records_exact_digest() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/workbook.xlsx", b"old-workbook");
    let artifact_plan = ArtifactMaterializationPlan {
        artifact: EntityId::from_bytes([7; 16]),
        root: PathBuf::from("/project"),
        members: vec![ArtifactMaterializationMember {
            key: "package".into(),
            relative_path: PathBuf::from("workbook.xlsx"),
            expected_digest: Some(digest_bytes(b"old-workbook")),
            required: true,
            bytes: Some(b"new-workbook".to_vec()),
        }],
        opaque_content: Vec::new(),
    };

    let receipt = materialize_artifact(&filesystem, &artifact_plan, None)
        .expect("atomically replace one artifact member");

    assert_eq!(receipt.state, ArtifactMaterializationState::Complete);
    assert!(receipt.members[0].committed);
    assert_eq!(
        receipt.members[0].output_digest,
        Some(digest_bytes(b"new-workbook"))
    );
    assert_eq!(*filesystem.writes.lock().expect("writes"), 1);
    assert_eq!(
        filesystem
            .read(std::path::Path::new("/project/workbook.xlsx"))
            .expect("replacement bytes"),
        b"new-workbook"
    );
}

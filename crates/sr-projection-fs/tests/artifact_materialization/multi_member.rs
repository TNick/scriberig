//! Verifies complete replacement of all members in one artifact plan.

use super::*;

/// Replaces every member of a multi-file artifact and records exact success.
#[test]
fn multi_member_success_replaces_and_receipts_every_member() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"old-shape");
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let plan = plan();

    let receipt = materialize_artifact(&filesystem, &plan, None).expect("materialize artifact");

    assert_eq!(receipt.state, ArtifactMaterializationState::Complete);
    assert_eq!(receipt.members.len(), 3);
    assert!(receipt.members.iter().all(|member| member.committed));
    assert!(
        receipt
            .members
            .iter()
            .all(|member| member.failure.is_none())
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shp"))
            .expect("shape"),
        b"new-shape"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shx"))
            .expect("index"),
        b"new-index"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.dbf"))
            .expect("table"),
        b"new-table"
    );
}

//! Verifies partial artifact failures leave recoverable receipts.

use super::*;

/// Leaves an explicit recoverable receipt when a required member write fails.
#[test]
fn partial_failure_is_recorded_and_retry_completes_idempotently() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"old-shape");
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    filesystem.fail_on(2);
    let plan = plan();
    let receipt = materialize_artifact(&filesystem, &plan, None).expect("write attempt receipt");
    assert_eq!(
        receipt.state,
        ArtifactMaterializationState::RecoverableFailure
    );
    assert_ne!(receipt.state, ArtifactMaterializationState::Complete);
    assert_eq!(receipt.members.len(), 3);
    assert!(receipt.members[0].committed);
    assert!(!receipt.members[1].committed);
    assert!(receipt.members[1].failure.is_some());
    assert!(receipt.members[2].committed);
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shx"))
            .expect("index remains unchanged after partial failure"),
        b"old-index"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.dbf"))
            .expect("later table write completes despite an earlier member failure"),
        b"new-table"
    );

    *filesystem.fail_write.lock().expect("failure") = None;
    let recovered = materialize_artifact(&filesystem, &plan, Some(&receipt))
        .expect("recover partial materialization");
    assert_eq!(recovered.state, ArtifactMaterializationState::Complete);
    assert!(recovered.members.iter().all(|member| member.committed));
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shx"))
            .expect("index"),
        b"new-index"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.shp"))
            .expect("shape"),
        b"new-shape"
    );
    assert_eq!(
        filesystem
            .read(Path::new("/project/roads.dbf"))
            .expect("table"),
        b"new-table"
    );
}

/// Recovers when a member write succeeds before its progress receipt is durable.
#[test]
fn committed_member_recovers_after_progress_persistence_failure() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/workbook.xlsx", b"old-workbook");
    let plan = ArtifactMaterializationPlan {
        artifact: EntityId::from_bytes([13; 16]),
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
    let mut durable = None;
    let failed = materialize_artifact_validated_with_progress(
        &filesystem,
        &plan,
        None,
        |_| Ok(()),
        |receipt| {
            if receipt.members[0].committed {
                Err(IoError::Write(std::io::Error::other(
                    "injected receipt failure",
                )))
            } else {
                durable = Some(receipt.clone());
                Ok(())
            }
        },
    );
    assert!(failed.is_err());
    let retained = durable.expect("latest durable attempt receipt");

    let recovered = materialize_artifact_validated_with_progress(
        &filesystem,
        &plan,
        Some(&retained),
        |_| Ok(()),
        |_| Ok(()),
    )
    .expect("recover committed member");

    assert_eq!(recovered.state, ArtifactMaterializationState::Complete);
    assert!(recovered.members[0].committed);
    assert_eq!(*filesystem.writes.lock().expect("writes"), 1);
}

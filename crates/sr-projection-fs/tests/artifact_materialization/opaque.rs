//! Verifies materialization enforces preserved opaque payload evidence.

use sr_projection_fs::{
    ArtifactMaterializationState, IoError, OpaqueContentOutcome, digest_bytes,
    discard_opaque_record, materialize_artifact, preserve_embedded_opaque_record,
    preserve_opaque_record,
};

use super::{FakeFileSystem, plan};

/// Binds nested opaque evidence to the exact staged container bytes.
#[test]
fn embedded_opaque_content_is_bound_to_exact_output_container() {
    let filesystem = FakeFileSystem::default();
    let source = b"old-container";
    let output = b"new-container-with-compressed-member";
    filesystem.seed("/project/roads.shp", source);
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let mut artifact_plan = plan();
    artifact_plan.members[0].expected_digest = Some(digest_bytes(source));
    artifact_plan.members[0].bytes = Some(output.to_vec());
    let record = preserve_embedded_opaque_record(
        "package-part:word/header1.xml",
        "geometry",
        source,
        b"unchanged nested bytes",
        output,
        b"unchanged nested bytes",
    )
    .expect("capture embedded preservation proof");
    artifact_plan.opaque_content = vec![OpaqueContentOutcome::Preserved(record)];

    let receipt = materialize_artifact(&filesystem, &artifact_plan, None)
        .expect("materialize proven embedded content");
    assert_eq!(receipt.state, ArtifactMaterializationState::Complete);
    assert_eq!(receipt.opaque_content, artifact_plan.opaque_content);

    artifact_plan.members[0].bytes = Some(b"different output container".to_vec());
    assert!(matches!(
        materialize_artifact(&filesystem, &artifact_plan, None),
        Err(IoError::UnsafeArtifactPlan)
    ));
}

/// Rejects a compound write that omits source-bound opaque content.
#[test]
fn required_opaque_content_must_survive_or_have_discard_provenance() {
    let filesystem = FakeFileSystem::default();
    filesystem.seed("/project/roads.shp", b"known=old\nopaque=extension\n");
    filesystem.seed("/project/roads.shx", b"old-index");
    filesystem.seed("/project/roads.dbf", b"old-table");
    let mut artifact_plan = plan();
    artifact_plan.members[0].expected_digest = Some(digest_bytes(b"known=old\nopaque=extension\n"));
    artifact_plan.members[0].bytes = Some(b"known=new\nopaque=extension\n".to_vec());
    let record = preserve_opaque_record(
        "extension",
        "geometry",
        b"known=old\nopaque=extension\n",
        b"opaque=extension\n",
    )
    .expect("capture opaque source evidence");
    artifact_plan.opaque_content = vec![OpaqueContentOutcome::Preserved(record)];

    let completed = materialize_artifact(&filesystem, &artifact_plan, None)
        .expect("materialize preserved opaque content");
    assert_eq!(completed.state, ArtifactMaterializationState::Complete);

    artifact_plan.members[0].expected_digest = Some(digest_bytes(b"known=old\nopaque=extension\n"));
    artifact_plan.members[0].bytes = Some(b"known=changed\nopaque=lost\n".to_vec());
    assert!(matches!(
        materialize_artifact(&filesystem, &artifact_plan, None),
        Err(IoError::UnsafeArtifactPlan)
    ));
    artifact_plan.opaque_content = vec![
        discard_opaque_record(
            "extension",
            "geometry",
            digest_bytes(b"known=old\nopaque=extension\n"),
            "delete owning record",
        )
        .expect("record deliberate removal"),
    ];
    let discard_filesystem = FakeFileSystem::default();
    discard_filesystem.seed("/project/roads.shp", b"known=old\nopaque=extension\n");
    discard_filesystem.seed("/project/roads.shx", b"old-index");
    discard_filesystem.seed("/project/roads.dbf", b"old-table");
    let discarded = materialize_artifact(&discard_filesystem, &artifact_plan, None)
        .expect("materialize explicit opaque discard");
    assert_eq!(discarded.state, ArtifactMaterializationState::Complete);
    assert_eq!(discarded.opaque_content, artifact_plan.opaque_content);
}

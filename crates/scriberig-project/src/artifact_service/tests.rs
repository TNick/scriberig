//! Tests exact-source fidelity gates used by rich artifact candidate creation.

use std::fs;
use std::sync::Arc;

use scriberig_store_sqlite::SqliteStore;

use super::{ArtifactSynchronizationState, synchronization_for_artifact};
use crate::{
    ArtifactCapability, ArtifactFidelity, ArtifactId, ArtifactKind, ArtifactManifest,
    ArtifactMemberDescriptor, ArtifactMemberKey, ArtifactSourceEvidence, OpaquePreservation,
    ProjectService,
};
use scriberig_core::{EntityId, ProjectId};

/// Rejects an invalid package whose shared fidelity is read-only.
#[test]
fn editable_source_gate_rejects_read_only_and_stale_members() {
    let root = tempfile::tempdir().expect("temporary project");
    fs::write(root.path().join("broken.xlsx"), b"not an XLSX package")
        .expect("write invalid package");
    let database = root.path().with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, root.path()).expect("scan package");
    let source = store
        .project_files(scan.project.id)
        .expect("persist source")
        .remove(0);

    let read_only = service
        .require_editable_source(scan.project.id, &source.relative_path, source.digest)
        .expect_err("read-only artifact cannot be edited");
    assert_eq!(read_only.code, "artifact_read_only");
    assert!(read_only.message.contains("xlsx"));
    let stale = service
        .require_editable_source(scan.project.id, &source.relative_path, [0; 32])
        .expect_err("stale capability evidence cannot be used");
    assert_eq!(stale.code, "artifact_fidelity_stale");

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
}

/// Classifies exact live artifacts as synchronized, modified, or missing.
#[test]
fn synchronization_status_compares_live_members_to_exact_baseline() {
    let root = tempfile::tempdir().expect("temporary project");
    let path = root.path().join("roads.shp");
    fs::write(&path, b"baseline member").expect("write baseline");
    let baseline_member = blake3::hash(b"baseline member");
    let artifact = ArtifactManifest {
        id: ArtifactId::from_source_entity(EntityId::from_bytes([1; 16])),
        logical_key: "roads".to_owned(),
        kind: ArtifactKind::Shapefile,
        members: vec![ArtifactMemberDescriptor {
            key: ArtifactMemberKey("geometry".to_owned()),
            project_path: Some("roads.shp".to_owned()),
            container_part: None,
            digest: *baseline_member.as_bytes(),
        }],
        source: ArtifactSourceEvidence {
            project: ProjectId::from_bytes([2; 16]),
            scan_id: [3; 32],
            adapter: "shapefile".to_owned(),
            adapter_version: None,
            classification: "geospatial".to_owned(),
        },
        fidelity: ArtifactFidelity {
            capability: ArtifactCapability::ReadOnlyDerived,
            opaque_preservation: OpaquePreservation::Preserved,
            diagnostic: None,
        },
        references: Vec::new(),
    };

    let synchronized = synchronization_for_artifact(root.path(), &artifact);
    assert_eq!(synchronized.status, ArtifactSynchronizationState::InSync);
    assert_eq!(
        synchronized.baseline_digest,
        synchronized.observed_digest.unwrap()
    );

    fs::write(&path, b"external change").expect("change live member");
    let modified = synchronization_for_artifact(root.path(), &artifact);
    assert_eq!(modified.status, ArtifactSynchronizationState::Modified);
    assert_ne!(modified.baseline_digest, modified.observed_digest.unwrap());

    fs::remove_file(path).expect("remove live member");
    let missing = synchronization_for_artifact(root.path(), &artifact);
    assert_eq!(missing.status, ArtifactSynchronizationState::Missing);
    assert!(missing.observed_digest.is_none());

    let mut escaping = artifact;
    escaping.members[0].project_path = Some("../outside.shp".to_owned());
    let unsafe_member = synchronization_for_artifact(root.path(), &escaping);
    assert_eq!(
        unsafe_member.status,
        ArtifactSynchronizationState::Unavailable
    );
}

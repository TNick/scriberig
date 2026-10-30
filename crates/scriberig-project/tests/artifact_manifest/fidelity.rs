//! Verifies artifact editability and preservation fidelity classifications.

use std::fs;
use std::sync::Arc;

use scriberig_project::{ArtifactCapability, ArtifactKind, OpaquePreservation, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Publishes XLSX fidelity from exact package capability and safe diagnostics.
#[test]
fn xlsx_artifact_fidelity_uses_persisted_package_capability() {
    let root = super::temporary_project();
    let mut workbook = rxls::Workbook::new();
    workbook.add_sheet("Budget").write_number(0, 0, 10.0);
    let valid = workbook.to_xlsx_checked().expect("valid XLSX package");
    fs::write(root.join("editable.xlsx"), valid).expect("write editable package");
    fs::write(root.join("invalid.xlsx"), b"not an XLSX package").expect("write invalid package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan packages");
    let artifacts = service
        .artifacts(scan.project.id)
        .expect("artifact fidelity");
    let editable = artifacts
        .iter()
        .find(|artifact| artifact.logical_key == "xlsx:editable.xlsx")
        .expect("editable XLSX artifact");
    assert_eq!(
        editable.fidelity.capability,
        ArtifactCapability::EditableWithOpaquePreservation
    );
    assert_eq!(
        editable.fidelity.opaque_preservation,
        OpaquePreservation::Preserved
    );
    assert_eq!(editable.fidelity.diagnostic, None);
    assert_eq!(editable.kind, ArtifactKind::Xlsx);
    assert_eq!(
        editable
            .members
            .iter()
            .filter(|member| member.project_path.is_some())
            .count(),
        1
    );
    assert_eq!(
        editable
            .members
            .iter()
            .find(|member| member.project_path.is_some())
            .and_then(|member| member.project_path.as_deref()),
        Some("editable.xlsx")
    );
    assert_eq!(
        editable.source.adapter_version.as_deref(),
        Some(scriberig_domain_xlsx::ADAPTER_VERSION)
    );
    let invalid = artifacts
        .iter()
        .find(|artifact| artifact.logical_key == "xlsx:invalid.xlsx")
        .expect("invalid XLSX artifact");
    assert_eq!(invalid.fidelity.capability, ArtifactCapability::InspectOnly);
    assert_eq!(
        invalid.fidelity.opaque_preservation,
        OpaquePreservation::Unproven
    );
    assert!(invalid.fidelity.diagnostic.is_some());

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

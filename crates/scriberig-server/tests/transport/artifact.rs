//! Verifies artifact fidelity survives the shared application and protocol boundary.

use std::collections::BTreeSet;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, StoreInspectionBackend,
};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_server::InProcessTransport;
use scriberig_store_sqlite::SqliteStore;

/// Preserves conservative artifact fidelity through the remote client contract.
#[test]
fn inspect_artifact_round_trip_preserves_fidelity_and_diagnostic() {
    let root = std::env::temp_dir().join(format!("scriberig-remote-artifact-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    std::fs::write(root.join("unknown.payload"), [0, 255, 1, 2]).expect("write opaque bytes");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan project").project.id;
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let capabilities = CapabilityContext::with_grants([grant]);
    let local = service
        .inspect_artifacts(
            &ApplicationContext {
                capabilities: capabilities.clone(),
                ..ApplicationContext::default()
            },
            project,
            0,
            10,
        )
        .expect("embedded application artifact page");
    let client =
        Client::new(InProcessTransport::new(service.clone()).with_capabilities(capabilities));

    let page = client
        .inspect_artifacts(project.to_string(), 0, 10)
        .expect("inspect artifact page");
    let artifact = page.artifacts.first().expect("opaque artifact summary");
    let local_artifact = local.artifacts.first().expect("embedded artifact summary");
    assert_eq!(page.project, project.to_string());
    assert_eq!(page.total, local.total);
    assert_eq!(artifact.id, local_artifact.id.entity().to_string());
    assert_eq!(artifact.logical_key, local_artifact.logical_key);
    assert_eq!(artifact.members.len(), local_artifact.members.len());
    assert_eq!(artifact.members[0].key, local_artifact.members[0].key.0);
    assert_eq!(
        artifact.members[0].digest,
        hex_bytes(&local_artifact.members[0].digest)
    );
    assert_eq!(artifact.fidelity, "read-only-derived");
    assert_eq!(artifact.opaque_preservation, "unproven");
    assert_eq!(artifact.synchronization_status, "in-sync");
    assert_eq!(
        artifact.baseline_digest,
        artifact.observed_digest.as_deref().unwrap()
    );
    assert!(
        artifact
            .diagnostic
            .as_deref()
            .is_some_and(|value| { value.contains("No artifact writer") })
    );
    let denied =
        Client::new(InProcessTransport::new(service)).inspect_artifacts(project.to_string(), 0, 10);
    assert!(denied.is_err(), "ungranted remote read must fail closed");

    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}

/// Encodes bytes as lower-case hexadecimal for comparison with the protocol DTO.
///
/// - `bytes`: Exact digest bytes from the application manifest.
///
/// The normalized digest text.
///
/// # Returns
/// Two lower-case hexadecimal characters for each byte.
fn hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(result, "{byte:02x}");
    }
    result
}

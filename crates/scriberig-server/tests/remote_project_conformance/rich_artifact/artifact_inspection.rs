//! Verifies artifact fidelity and diagnostics match across application and client APIs.

use scriberig_app::ApplicationContext;
use scriberig_core::ProjectId;

use super::{EmbeddedApplication, RemoteApplicationClient};

/// Compares bounded artifact capability facts across embedded and remote reads.
///
/// - `embedded`: Shared application service used as the canonical reference.
/// - `remote`: Public client using the in-process transport.
/// - `context`: Exact Inspect grant used by both requests.
/// - `project`: Project whose persisted artifact scan is compared.
pub(super) fn assert_artifact_fidelity_matches(
    embedded: &EmbeddedApplication,
    remote: &RemoteApplicationClient,
    context: &ApplicationContext,
    project: ProjectId,
) {
    let local = embedded
        .inspect_artifacts(context, project, 0, 100)
        .expect("embedded artifact page");
    let remote = remote
        .inspect_artifacts(project.to_string(), 0, 100)
        .expect("remote artifact page");
    assert_eq!(remote.project, project.to_string());
    assert_eq!(remote.total, local.total);
    assert_eq!(remote.artifacts.len(), local.artifacts.len());

    for artifact in local.artifacts {
        let summary = remote
            .artifacts
            .iter()
            .find(|summary| summary.id == artifact.id.entity().to_string())
            .expect("remote artifact with the same semantic identity");
        assert_eq!(summary.logical_key, artifact.logical_key);
        assert_eq!(summary.adapter, artifact.source.adapter);
        assert_eq!(summary.adapter_version, artifact.source.adapter_version);
        let sync = local
            .synchronization
            .iter()
            .find(|status| status.artifact == artifact.id.entity())
            .expect("embedded synchronization status");
        assert_eq!(summary.baseline_digest, sync.baseline_digest);
        assert_eq!(summary.observed_digest, sync.observed_digest);
        assert_eq!(summary.synchronization_status, "in-sync");
        assert_eq!(
            summary.fidelity,
            fidelity_label(artifact.fidelity.capability)
        );
        assert_eq!(
            summary.opaque_preservation,
            opaque_preservation_label(artifact.fidelity.opaque_preservation)
        );
        assert_eq!(summary.diagnostic, artifact.fidelity.diagnostic);
        assert_eq!(summary.member_count, artifact.members.len());
    }
}

/// Converts internal edit capability to its stable wire label.
///
/// - `capability`: Shared project fidelity capability.
///
/// # Returns
/// Stable protocol label for the capability.
fn fidelity_label(capability: scriberig_project::ArtifactCapability) -> &'static str {
    match capability {
        scriberig_project::ArtifactCapability::Unsupported => "unsupported",
        scriberig_project::ArtifactCapability::InspectOnly => "inspect-only",
        scriberig_project::ArtifactCapability::ReadOnlyDerived => "read-only-derived",
        scriberig_project::ArtifactCapability::LosslessEditable => "lossless-editable",
        scriberig_project::ArtifactCapability::LossAwareEditable => "loss-aware-editable",
        scriberig_project::ArtifactCapability::EditableWithOpaquePreservation => {
            "editable-with-opaque-preservation"
        }
        scriberig_project::ArtifactCapability::RestrictedExternalProviderEdit => {
            "restricted-external-provider-edit"
        }
    }
}

/// Converts internal opaque-preservation status to its stable wire label.
///
/// - `preservation`: Shared project opaque-content preservation status.
///
/// # Returns
/// Stable protocol label for the preservation status.
fn opaque_preservation_label(preservation: scriberig_project::OpaquePreservation) -> &'static str {
    match preservation {
        scriberig_project::OpaquePreservation::NotPresent => "not-present",
        scriberig_project::OpaquePreservation::Preserved => "preserved",
        scriberig_project::OpaquePreservation::Unproven => "unproven",
        scriberig_project::OpaquePreservation::NotPreserved => "not-preserved",
    }
}

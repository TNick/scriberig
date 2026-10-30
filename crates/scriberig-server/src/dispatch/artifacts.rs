//! Maps authorized application artifact pages to stable wire DTOs.

use scriberig_app::ApplicationContext;
use scriberig_protocol::{
    ArtifactInspectRequest, ArtifactInspectResponse, ArtifactMemberSummary, ArtifactMembersRequest,
    ArtifactMembersResponse, ArtifactReference, ArtifactSummary, Response,
};

use crate::server::ApplicationService;

/// Inspects one bounded artifact page through the shared application service.
///
/// - `service`: Application boundary receiving the scoped request.
/// - `request`: Project and bounded page selection.
/// - `correlation_id`: Protocol request correlation identity.
///
/// # Returns
/// Typed artifact summary page or a structured application error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ArtifactInspectRequest,
    correlation_id: &str,
) -> Response {
    service
        .inspect_artifacts(context, &request.project, request.offset, request.limit)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |page| {
                Response::ArtifactInspect(ArtifactInspectResponse {
                    project: page.project.to_string(),
                    artifacts: page
                        .artifacts
                        .into_iter()
                        .zip(page.synchronization)
                        .map(|(manifest, status)| map_artifact(manifest, status))
                        .collect(),
                    total: page.total,
                    next_offset: page.next_offset,
                })
            },
        )
}

/// Reads one bounded page of exact physical and package-part member evidence.
///
/// - `service`: Application boundary receiving the scoped request.
/// - `context`: Trusted request context with project Inspect authority.
/// - `request`: Project, artifact identity, and bounded page selection.
/// - `correlation_id`: Protocol request correlation identity.
///
/// # Returns
/// Typed member page or a structured application error.
pub(super) fn members<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ArtifactMembersRequest,
    correlation_id: &str,
) -> Response {
    service
        .inspect_artifact_members(
            context,
            &request.project,
            &request.artifact,
            request.offset,
            request.limit,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |page| {
                Response::ArtifactMembers(ArtifactMembersResponse {
                    project: page.project.to_string(),
                    artifact: page.artifact.to_string(),
                    members: page.members.into_iter().map(map_member).collect(),
                    total: page.total,
                    next_offset: page.next_offset,
                })
            },
        )
}

/// Projects an internal artifact manifest into its stable public summary.
///
/// - `manifest`: Exact persisted artifact summary to expose.
/// - `synchronization`: Exact baseline and current observation status.
///
/// # Returns
/// Wire-safe artifact metadata with no source bytes.
fn map_artifact(
    manifest: scriberig_project::ArtifactManifest,
    synchronization: scriberig_project::ArtifactSynchronizationStatus,
) -> ArtifactSummary {
    let member_count = manifest.members.len();
    ArtifactSummary {
        id: manifest.id.entity().to_string(),
        logical_key: manifest.logical_key,
        kind: match manifest.kind {
            scriberig_project::ArtifactKind::Adapter(kind)
            | scriberig_project::ArtifactKind::SingleFile(kind) => kind,
            scriberig_project::ArtifactKind::Shapefile => "shapefile".into(),
            scriberig_project::ArtifactKind::Xlsx => "xlsx".into(),
            scriberig_project::ArtifactKind::Docx => "docx".into(),
        },
        adapter: manifest.source.adapter,
        adapter_version: manifest.source.adapter_version,
        scan_id: encode_hex(&manifest.source.scan_id),
        baseline_digest: synchronization.baseline_digest,
        observed_digest: synchronization.observed_digest,
        synchronization_status: match synchronization.status {
            scriberig_project::ArtifactSynchronizationState::InSync => "in-sync",
            scriberig_project::ArtifactSynchronizationState::Modified => "modified",
            scriberig_project::ArtifactSynchronizationState::Missing => "missing",
            scriberig_project::ArtifactSynchronizationState::Unavailable => "unavailable",
        }
        .to_owned(),
        fidelity: match manifest.fidelity.capability {
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
        .into(),
        opaque_preservation: match manifest.fidelity.opaque_preservation {
            scriberig_project::OpaquePreservation::NotPresent => "not-present",
            scriberig_project::OpaquePreservation::Preserved => "preserved",
            scriberig_project::OpaquePreservation::Unproven => "unproven",
            scriberig_project::OpaquePreservation::NotPreserved => "not-preserved",
        }
        .into(),
        diagnostic: manifest.fidelity.diagnostic,
        references: manifest
            .references
            .into_iter()
            .map(|reference| match reference {
                scriberig_project::ArtifactExternalReference::ProjectPath(path) => {
                    ArtifactReference {
                        kind: "project-path".into(),
                        target: path,
                    }
                }
                scriberig_project::ArtifactExternalReference::Uri { scheme, value } => {
                    ArtifactReference {
                        kind: format!("uri:{scheme}"),
                        target: value,
                    }
                }
                scriberig_project::ArtifactExternalReference::Artifact(id) => ArtifactReference {
                    kind: "artifact".into(),
                    target: id.entity().to_string(),
                },
            })
            .collect(),
        members: manifest
            .members
            .into_iter()
            .filter(|member| member.project_path.is_some())
            .take(20)
            .map(map_member)
            .collect(),
        member_count,
    }
}

/// Projects exact internal member evidence into its wire representation.
///
/// - `member`: Exact persisted member descriptor.
///
/// # Returns
/// Wire-safe member metadata and its digest.
fn map_member(member: scriberig_project::ArtifactMemberDescriptor) -> ArtifactMemberSummary {
    ArtifactMemberSummary {
        key: member.key.0,
        project_path: member.project_path,
        container_part: member.container_part,
        digest: encode_hex(&member.digest),
    }
}

/// Converts exact bytes into lower-case hexadecimal text.
///
/// - `bytes`: Exact value to encode.
///
/// # Returns
/// Two hexadecimal characters for every input byte.
fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

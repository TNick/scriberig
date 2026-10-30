//! Projects bounded artifact metadata into terminal-safe semantic rows.

const LOWER_HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Bounded artifact summaries shared by local and remote TUI service adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPageModel {
    /// Stable project identity.
    pub project: String,
    /// Bounded artifacts in deterministic order.
    pub artifacts: Vec<ArtifactRow>,
    /// Total artifacts in the persisted scan.
    pub total: usize,
    /// Offset for the next bounded page.
    pub next_offset: Option<usize>,
}

/// Terminal-safe artifact detail with no inline source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRow {
    /// Stable semantic artifact identity.
    pub id: String,
    /// Adapter-owned logical artifact key.
    pub logical_key: String,
    /// Artifact kind label.
    pub kind: String,
    /// Source adapter identifier and optional version.
    pub adapter: String,
    /// Persisted scan identity.
    pub scan_id: String,
    /// Aggregate exact persisted member baseline digest.
    pub baseline_digest: String,
    /// Aggregate live member digest when all members were readable.
    pub observed_digest: Option<String>,
    /// Live synchronization state against the persisted baseline.
    pub synchronization_status: String,
    /// Shared fidelity/editability label.
    pub fidelity: String,
    /// Opaque-content preservation guarantee.
    pub opaque_preservation: String,
    /// Safe restriction or capability diagnostic.
    pub diagnostic: Option<String>,
    /// Bounded member descriptors.
    pub members: Vec<ArtifactMemberRow>,
    /// Total physical and container-part member count.
    pub member_count: usize,
    /// Typed reference summaries.
    pub references: Vec<String>,
}

/// One safe logical or physical artifact member descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMemberRow {
    /// Adapter-owned stable member key.
    pub key: String,
    /// Project-relative path, when the member is a filesystem file.
    pub project_path: Option<String>,
    /// Container-internal part path, when applicable.
    pub container_part: Option<String>,
    /// Exact source member digest.
    pub digest: String,
}

impl ArtifactPageModel {
    /// Converts the shared application artifact page into terminal-safe rows.
    ///
    /// # Parameters
    /// - `page`: Authorized bounded application result.
    ///
    /// # Returns
    /// A presentation model retaining fidelity, provenance, members, and references.
    #[must_use]
    pub fn from_application(page: scriberig_app::ArtifactManifestPage) -> Self {
        Self {
            project: page.project.to_string(),
            artifacts: page
                .artifacts
                .into_iter()
                .zip(page.synchronization)
                .map(|(manifest, synchronization)| {
                    artifact_from_manifest(manifest, synchronization)
                })
                .collect(),
            total: page.total,
            next_offset: page.next_offset,
        }
    }

    /// Converts the public remote artifact DTO to the same terminal-safe row model.
    ///
    /// # Parameters
    /// - `page`: Typed protocol response from the remote client.
    ///
    /// # Returns
    /// A presentation model equivalent to the local application mapping.
    #[must_use]
    pub fn from_protocol(page: scriberig_protocol::ArtifactInspectResponse) -> Self {
        Self {
            project: page.project,
            artifacts: page
                .artifacts
                .into_iter()
                .map(|artifact| ArtifactRow {
                    id: artifact.id,
                    logical_key: artifact.logical_key,
                    kind: artifact.kind,
                    adapter: artifact
                        .adapter_version
                        .map_or(artifact.adapter.clone(), |version| {
                            format!("{} v{version}", artifact.adapter)
                        }),
                    scan_id: artifact.scan_id,
                    baseline_digest: artifact.baseline_digest,
                    observed_digest: artifact.observed_digest,
                    synchronization_status: artifact.synchronization_status,
                    fidelity: artifact.fidelity,
                    opaque_preservation: artifact.opaque_preservation,
                    diagnostic: artifact.diagnostic,
                    members: artifact
                        .members
                        .into_iter()
                        .map(|member| ArtifactMemberRow {
                            key: member.key,
                            project_path: member.project_path,
                            container_part: member.container_part,
                            digest: member.digest,
                        })
                        .collect(),
                    member_count: artifact.member_count,
                    references: artifact
                        .references
                        .into_iter()
                        .map(|reference| format!("{}: {}", reference.kind, reference.target))
                        .collect(),
                })
                .collect(),
            total: page.total,
            next_offset: page.next_offset,
        }
    }
}

/// Converts one internal manifest without exposing source bytes.
///
/// # Parameters
/// - `manifest`: Persisted logical artifact summary.
/// - `synchronization`: Persisted baseline and external observation status.
///
/// # Returns
/// A terminal-safe artifact row with fidelity and provenance labels.
fn artifact_from_manifest(
    manifest: scriberig_project::ArtifactManifest,
    synchronization: scriberig_project::ArtifactSynchronizationStatus,
) -> ArtifactRow {
    let member_count = manifest.members.len();
    ArtifactRow {
        id: manifest.id.entity().to_string(),
        logical_key: manifest.logical_key,
        kind: match manifest.kind {
            scriberig_project::ArtifactKind::Adapter(kind)
            | scriberig_project::ArtifactKind::SingleFile(kind) => kind,
            scriberig_project::ArtifactKind::Shapefile => "shapefile".to_owned(),
            scriberig_project::ArtifactKind::Xlsx => "xlsx".to_owned(),
            scriberig_project::ArtifactKind::Docx => "docx".to_owned(),
        },
        adapter: manifest
            .source
            .adapter_version
            .map_or(manifest.source.adapter.clone(), |version| {
                format!("{} v{version}", manifest.source.adapter)
            }),
        scan_id: encode_digest(&manifest.source.scan_id),
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
        .to_owned(),
        opaque_preservation: match manifest.fidelity.opaque_preservation {
            scriberig_project::OpaquePreservation::NotPresent => "not-present",
            scriberig_project::OpaquePreservation::Preserved => "preserved",
            scriberig_project::OpaquePreservation::Unproven => "unproven",
            scriberig_project::OpaquePreservation::NotPreserved => "not-preserved",
        }
        .to_owned(),
        diagnostic: manifest.fidelity.diagnostic,
        members: manifest
            .members
            .into_iter()
            .filter(|member| member.project_path.is_some())
            .take(20)
            .map(|member| ArtifactMemberRow {
                key: member.key.0,
                project_path: member.project_path,
                container_part: member.container_part,
                digest: encode_digest(&member.digest),
            })
            .collect(),
        member_count,
        references: manifest
            .references
            .into_iter()
            .map(|reference| match reference {
                scriberig_project::ArtifactExternalReference::ProjectPath(path) => {
                    format!("project-path: {path}")
                }
                scriberig_project::ArtifactExternalReference::Uri { scheme, value } => {
                    format!("uri:{scheme}: {value}")
                }
                scriberig_project::ArtifactExternalReference::Artifact(id) => {
                    format!("artifact: {}", id.entity())
                }
            })
            .collect(),
    }
}

/// Encodes exact digest bytes using lowercase hexadecimal.
///
/// # Parameters
/// - `digest`: Fixed-width content digest.
///
/// # Returns
/// A stable lowercase hexadecimal representation.
pub(super) fn encode_digest(digest: &[u8; 32]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(LOWER_HEX_DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(LOWER_HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::ArtifactPageModel;
    use scriberig_core::{EntityId, ProjectId};
    use scriberig_project::{
        ArtifactCapability, ArtifactExternalReference, ArtifactFidelity, ArtifactId, ArtifactKind,
        ArtifactManifest, ArtifactMemberDescriptor, ArtifactMemberKey, ArtifactSourceEvidence,
        ArtifactSynchronizationState, ArtifactSynchronizationStatus, OpaquePreservation,
    };

    /// Creates a deterministic artifact identity for transport mapping fixtures.
    ///
    /// # Parameters
    /// - `byte`: Repeated identity byte.
    ///
    /// # Returns
    /// Stable artifact identity backed by a Scriberig entity ID.
    fn artifact_id(byte: u8) -> ArtifactId {
        ArtifactId::from_source_entity(EntityId::from_bytes([byte; 16]))
    }

    /// Keeps empty local and remote artifact pages equivalent at the TUI boundary.
    #[test]
    fn local_and_remote_empty_pages_have_the_same_view_model() {
        let project = ProjectId::from_bytes([7; 16]);
        let local = ArtifactPageModel::from_application(scriberig_app::ArtifactManifestPage {
            project,
            artifacts: Vec::new(),
            synchronization: Vec::new(),
            total: 0,
            next_offset: None,
        });
        let remote =
            ArtifactPageModel::from_protocol(scriberig_protocol::ArtifactInspectResponse {
                project: project.to_string(),
                artifacts: Vec::new(),
                total: 0,
                next_offset: None,
            });
        assert_eq!(local, remote);
    }

    /// Keeps provenance, fidelity, member, and reference fields equivalent across transports.
    #[test]
    fn local_and_remote_artifact_details_normalize_identically() {
        let project = ProjectId::from_bytes([7; 16]);
        let manifest = ArtifactManifest {
            id: artifact_id(10),
            logical_key: "roads".to_owned(),
            kind: ArtifactKind::Shapefile,
            members: vec![ArtifactMemberDescriptor {
                key: ArtifactMemberKey("geometry".to_owned()),
                project_path: Some("roads.shp".to_owned()),
                container_part: None,
                digest: [8; 32],
            }],
            source: ArtifactSourceEvidence {
                project,
                scan_id: [9; 32],
                adapter: "shapefile".to_owned(),
                adapter_version: Some("2".to_owned()),
                classification: "geospatial".to_owned(),
            },
            fidelity: ArtifactFidelity {
                capability: ArtifactCapability::EditableWithOpaquePreservation,
                opaque_preservation: OpaquePreservation::Preserved,
                diagnostic: None,
            },
            references: vec![ArtifactExternalReference::Artifact(artifact_id(11))],
        };
        let local = ArtifactPageModel::from_application(scriberig_app::ArtifactManifestPage {
            project,
            artifacts: vec![manifest],
            synchronization: vec![ArtifactSynchronizationStatus {
                artifact: artifact_id(10).entity(),
                baseline_digest: "aa".repeat(32),
                observed_digest: Some("aa".repeat(32)),
                status: ArtifactSynchronizationState::InSync,
            }],
            total: 1,
            next_offset: None,
        });
        let remote =
            ArtifactPageModel::from_protocol(scriberig_protocol::ArtifactInspectResponse {
                project: project.to_string(),
                artifacts: vec![scriberig_protocol::ArtifactSummary {
                    id: artifact_id(10).entity().to_string(),
                    logical_key: "roads".to_owned(),
                    kind: "shapefile".to_owned(),
                    adapter: "shapefile".to_owned(),
                    adapter_version: Some("2".to_owned()),
                    scan_id: "09".repeat(32),
                    baseline_digest: "aa".repeat(32),
                    observed_digest: Some("aa".repeat(32)),
                    synchronization_status: "in-sync".to_owned(),
                    fidelity: "editable-with-opaque-preservation".to_owned(),
                    opaque_preservation: "preserved".to_owned(),
                    diagnostic: None,
                    references: vec![scriberig_protocol::ArtifactReference {
                        kind: "artifact".to_owned(),
                        target: artifact_id(11).entity().to_string(),
                    }],
                    members: vec![scriberig_protocol::ArtifactMemberSummary {
                        key: "geometry".to_owned(),
                        project_path: Some("roads.shp".to_owned()),
                        container_part: None,
                        digest: "08".repeat(32),
                    }],
                    member_count: 1,
                }],
                total: 1,
                next_offset: None,
            });
        assert_eq!(local, remote);
    }
}

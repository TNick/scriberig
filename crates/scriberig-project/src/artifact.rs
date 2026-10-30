//! Defines deterministic logical artifact manifests over persisted project scans.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::{CadProviderFormatCapability, CadProviderOpaquePreservation};
use serde::{Deserialize, Serialize};

#[path = "artifact/fidelity.rs"]
mod fidelity;
pub(crate) use fidelity::resolve_adapter_fidelity;
#[path = "artifact/container_members.rs"]
mod container_members;
pub(crate) use container_members::append_container_member_evidence;
#[path = "artifact/import.rs"]
mod import;
pub(crate) use import::append_artifact_revisions;

/// Identifies one logical artifact independently of its physical members.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtifactId(EntityId);

impl ArtifactId {
    /// Derives a distinct stable artifact identity from its primary source identity.
    ///
    /// # Parameters
    /// - `source`: Stable source entity identity, including unambiguous rename continuity.
    ///
    /// # Returns
    /// A namespaced artifact identity that survives source path changes.
    #[must_use]
    pub fn from_source_entity(source: EntityId) -> Self {
        let source_bytes = source.as_bytes();
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"scriberig.external-artifact.v1\0");
        hasher.update(source_bytes);
        let digest = hasher.finalize();
        let mut bytes = [0; 16];
        let digest_bytes = digest.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            bytes[index] = digest_bytes[index];
            index += 1;
        }
        Self(EntityId::from_bytes(bytes))
    }

    /// Returns the stable semantic entity identity for this artifact.
    #[must_use]
    pub const fn entity(self) -> EntityId {
        self.0
    }
}

/// Classifies the logical kind of a compound or single-file artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// An adapter-defined kind label.
    Adapter(String),
    /// A compound geospatial Shapefile dataset.
    Shapefile,
    /// An Office Open XML workbook package.
    Xlsx,
    /// An Office Open XML document package.
    Docx,
    /// A standalone file classified by its adapter.
    SingleFile(String),
}

/// Identifies a physical artifact member by its adapter-owned logical key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactMemberKey(pub String);

/// Describes a filesystem or container-internal member of an artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactMemberDescriptor {
    /// Logical member name stable within its artifact.
    pub key: ArtifactMemberKey,
    /// Project-relative filesystem path, absent for container-internal parts.
    pub project_path: Option<String>,
    /// Container-internal logical part path, absent for filesystem members.
    pub container_part: Option<String>,
    /// BLAKE3 digest of exact member bytes.
    pub digest: [u8; 32],
}

/// Records exact source bytes and their scan/import provenance.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactSourceEvidence {
    /// Project owning the scan evidence.
    pub project: ProjectId,
    /// Deterministic identity of the exact persisted scan manifest.
    pub scan_id: [u8; 32],
    /// Persisted scan adapter identifier.
    pub adapter: String,
    /// Adapter version when the adapter publishes one.
    pub adapter_version: Option<String>,
    /// Scan classification recorded with exact bytes.
    pub classification: String,
}

/// Represents an explicitly typed reference to another artifact or resource.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactExternalReference {
    /// A project-relative path reference.
    ProjectPath(String),
    /// A URI reference with its scheme retained separately.
    Uri {
        /// URI scheme, such as `https`.
        scheme: String,
        /// URI text after the scheme.
        value: String,
    },
    /// A semantic reference to another logical artifact.
    Artifact(ArtifactId),
}

/// Reports supported editability and opaque-content preservation guarantees.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactFidelity {
    /// Supported read/write capability level.
    pub capability: ArtifactCapability,
    /// Whether source-significant opaque content is preserved during edits.
    pub opaque_preservation: OpaquePreservation,
    /// Adapter-owned explanation for a restricted capability.
    pub diagnostic: Option<String>,
}

/// States the strongest supported semantic access mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactCapability {
    /// No artifact semantics are available.
    Unsupported,
    /// Source can be inspected without derived semantic access.
    InspectOnly,
    /// Read-only semantic data is available.
    ReadOnlyDerived,
    /// Edits are lossless for the supported schema.
    LosslessEditable,
    /// Edits are safely reconciled with known omissions.
    LossAwareEditable,
    /// Edits preserve unsupported opaque source content.
    EditableWithOpaquePreservation,
    /// Editing is delegated to a restricted external provider.
    RestrictedExternalProviderEdit,
}

/// States whether opaque source content survives semantic edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OpaquePreservation {
    /// No opaque content has been identified.
    NotPresent,
    /// Opaque bytes are retained and proven to round trip.
    Preserved,
    /// Opaque content exists but is not guaranteed to survive.
    Unproven,
    /// Opaque content is known to be lost by the writer.
    NotPreserved,
}

/// Maps optional CAD provider capabilities to the shared artifact fidelity model.
///
/// Provider claims are kept behind the project adapter boundary so the reusable
/// CAD domain never depends on project/application artifact infrastructure.
///
/// # Parameters
/// - `capability`: Provider capabilities for the exact CAD format.
///
/// # Returns
/// Shared fidelity assessment suitable for artifact summaries.
#[must_use]
pub fn cad_provider_artifact_fidelity(
    capability: &CadProviderFormatCapability,
) -> ArtifactFidelity {
    let (level, diagnostic) = if capability.write && capability.validate {
        match capability.opaque_preservation {
            CadProviderOpaquePreservation::Preserved => {
                (ArtifactCapability::EditableWithOpaquePreservation, None)
            }
            CadProviderOpaquePreservation::Unproven => (
                ArtifactCapability::RestrictedExternalProviderEdit,
                Some("provider output does not prove opaque content preservation"),
            ),
            CadProviderOpaquePreservation::NotPreserved => (
                ArtifactCapability::RestrictedExternalProviderEdit,
                Some("provider output is known to omit opaque content"),
            ),
        }
    } else if capability.write {
        (
            ArtifactCapability::RestrictedExternalProviderEdit,
            Some("provider write capability requires provider validation"),
        )
    } else if capability.read {
        (
            ArtifactCapability::ReadOnlyDerived,
            Some("provider supports reading but not writing this format"),
        )
    } else {
        (
            ArtifactCapability::InspectOnly,
            Some("provider exposes no semantic read or write capability"),
        )
    };
    ArtifactFidelity {
        capability: level,
        opaque_preservation: match capability.opaque_preservation {
            CadProviderOpaquePreservation::Preserved => OpaquePreservation::Preserved,
            CadProviderOpaquePreservation::Unproven => OpaquePreservation::Unproven,
            CadProviderOpaquePreservation::NotPreserved => OpaquePreservation::NotPreserved,
        },
        diagnostic: diagnostic.map(str::to_owned),
    }
}

/// Summarizes one logical artifact and the exact source members it contains.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Stable semantic artifact identity.
    pub id: ArtifactId,
    /// Logical artifact key independent of paths.
    pub logical_key: String,
    /// Adapter-independent artifact kind.
    pub kind: ArtifactKind,
    /// Exact source-member evidence in deterministic key order.
    pub members: Vec<ArtifactMemberDescriptor>,
    /// Source provenance for the persisted project scan.
    pub source: ArtifactSourceEvidence,
    /// Current adapter fidelity assessment.
    pub fidelity: ArtifactFidelity,
    /// Explicit references to external resources or artifacts.
    pub references: Vec<ArtifactExternalReference>,
}

/// Contains one bounded page of exact physical and container-part evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMemberPage {
    /// Stable artifact identity whose members are returned.
    pub artifact: ArtifactId,
    /// Exact member descriptors for this page.
    pub members: Vec<ArtifactMemberDescriptor>,
    /// Total member count in the exact artifact manifest.
    pub total: usize,
    /// Offset to request for the next page.
    pub next_offset: Option<usize>,
}

/// Explains why persisted source members could not be grouped safely.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactGroupingDiagnostic {
    /// Stable machine-readable ambiguity category.
    pub code: String,
    /// Candidate logical artifact key affected by the ambiguity.
    pub logical_key: String,
    /// Project-relative paths that caused the ambiguity.
    pub paths: Vec<String>,
    /// Safe explanation suitable for product surfaces.
    pub message: String,
}

impl ArtifactManifest {
    /// Encodes a manifest using deterministic JSON field and member ordering.
    ///
    /// # Parameters
    /// - `self`: Manifest to encode.
    ///
    /// # Returns
    /// Canonical JSON bytes.
    ///
    /// # Errors
    /// Returns a serialization error if the manifest cannot be encoded.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

#[path = "artifact/grouping.rs"]
mod grouping;
pub use grouping::{group_persisted_files, group_persisted_files_with_diagnostics};

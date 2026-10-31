//! Defines stable wire contracts for bounded rich-artifact inspection.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// States artifact edit support for snapshot-bound rich semantic views.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactEditCapability {
    /// The source format has no semantic artifact support.
    Unsupported,
    /// Exact source can be inspected without derived semantic data.
    InspectOnly,
    /// Derived semantic data is available without safe writes.
    ReadOnlyDerived,
    /// Supported edits preserve all modeled source semantics.
    LosslessEditable,
    /// Supported edits reconcile known source omissions safely.
    LossAwareEditable,
    /// Supported edits preserve source-significant opaque content.
    EditableWithOpaquePreservation,
    /// Editing uses a restricted external provider.
    RestrictedExternalProviderEdit,
}

/// Requests a bounded page of artifact summaries for one project.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactInspectRequest {
    /// Stable project identity.
    pub project: String,
    /// Zero-based artifact offset.
    pub offset: usize,
    /// Maximum number of artifact summaries requested.
    pub limit: usize,
}

/// Requests one bounded page of members for a stable artifact identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactMembersRequest {
    /// Stable project identity.
    pub project: String,
    /// Stable semantic artifact entity identity.
    pub artifact: String,
    /// Zero-based member offset.
    pub offset: usize,
    /// Maximum member descriptors requested.
    pub limit: usize,
}

/// Describes one physical or container-internal artifact member.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactMemberSummary {
    /// Adapter-owned logical member key.
    pub key: String,
    /// Project-relative source path, absent for package-internal members.
    pub project_path: Option<String>,
    /// Package-internal part name, absent for filesystem members.
    pub container_part: Option<String>,
    /// BLAKE3 digest of exact source bytes.
    pub digest: String,
}

/// Summarizes an artifact without embedding its source bytes.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactSummary {
    /// Stable semantic artifact identity.
    pub id: String,
    /// Adapter-owned logical key.
    pub logical_key: String,
    /// Stable artifact-kind label.
    pub kind: String,
    /// Persisted source adapter identifier.
    pub adapter: String,
    /// Adapter version when published.
    pub adapter_version: Option<String>,
    /// Deterministic digest identifying the exact source scan.
    pub scan_id: String,
    /// Aggregate exact digest of physical source members at the persisted baseline.
    #[serde(default)]
    pub baseline_digest: String,
    /// Aggregate digest of readable live members; absent when any member is unavailable.
    pub observed_digest: Option<String>,
    /// Live synchronization classification against the persisted baseline.
    #[serde(default = "unavailable_synchronization_status")]
    pub synchronization_status: String,
    /// Artifact editability/fidelity label.
    pub fidelity: String,
    /// Opaque preservation guarantee label.
    pub opaque_preservation: String,
    /// Safe adapter explanation when capability is restricted.
    pub diagnostic: Option<String>,
    /// Typed external resource references.
    pub references: Vec<ArtifactReference>,
    /// Exact source members represented by this artifact.
    ///
    /// This bounded summary contains filesystem members. Use the member-page
    /// operation for container-internal package parts.
    pub members: Vec<ArtifactMemberSummary>,
    /// Total physical and container-internal member count.
    pub member_count: usize,
}

/// Supplies a conservative synchronization value when reading older server responses.
///
/// # Returns
/// The status used when an older protocol payload has no synchronization data.
fn unavailable_synchronization_status() -> String {
    "unavailable".to_owned()
}

/// Returns one bounded page of physical or container-internal member evidence.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactMembersResponse {
    /// Stable project identity.
    pub project: String,
    /// Stable semantic artifact entity identity.
    pub artifact: String,
    /// Bounded exact member descriptors.
    pub members: Vec<ArtifactMemberSummary>,
    /// Total member count in the exact artifact manifest.
    pub total: usize,
    /// Offset for the next page, when present.
    pub next_offset: Option<usize>,
}

/// Represents one explicit artifact reference in a public summary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactReference {
    /// Stable reference-kind label.
    pub kind: String,
    /// Reference target, encoded according to its kind.
    pub target: String,
}

/// Returns a bounded page of project artifact summaries.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactInspectResponse {
    /// Stable project identity.
    pub project: String,
    /// Bounded logical artifact summaries.
    pub artifacts: Vec<ArtifactSummary>,
    /// Total logical artifact count in the persisted scan.
    pub total: usize,
    /// Offset for the next page, when present.
    pub next_offset: Option<usize>,
}

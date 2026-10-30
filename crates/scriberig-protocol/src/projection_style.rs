//! Defines reusable wire contracts for shared projection style semantics.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Names why a projection style is being resolved.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStylePurpose {
    /// Resolve presentation for one personal interactive view.
    InteractiveView,
    /// Resolve style for shared workspace materialization.
    SharedMaterialization,
    /// Resolve an explicitly selected export style.
    ExplicitExport,
}

/// Names the setting scope a style mutation targets.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStyleScope {
    /// The authenticated caller's personal preference.
    User,
    /// One personal view override.
    View {
        /// Stable identity of the view session.
        view_id: String,
    },
    /// Shared policy for one project.
    Project {
        /// Stable project identity.
        project_id: String,
    },
    /// Shared policy for one materialization workspace.
    Workspace {
        /// Stable workspace identity.
        workspace_id: String,
    },
}

/// Requests the adapter-owned schema for one projection kind.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSchemaRequest {
    /// Projection kind whose schema is requested.
    pub projection_kind: String,
}

/// Requests effective style resolution for an exact purpose and scope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleResolveRequest {
    /// Projection kind whose style is resolved.
    pub projection_kind: String,
    /// Stable project identity for shared policy and repository conventions.
    pub project_id: String,
    /// Stable workspace identity for materialization policy.
    pub workspace_id: String,
    /// Purpose controlling which style sources may apply.
    pub purpose: ProjectionStylePurpose,
    /// Personal view identity when resolving an interactive view.
    pub view_id: Option<String>,
    /// Explicit personal style bytes for an export request.
    pub explicit_personal: Option<Vec<u8>>,
}

/// Requests one setting write or clear in a named style scope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSettingRequest {
    /// Setting scope selected by the caller.
    pub scope: ProjectionStyleScope,
    /// Projection kind affected by the setting.
    pub projection_kind: String,
    /// Expected setting revision for compare-and-swap, when supplied.
    pub expected_revision: Option<u64>,
    /// Canonical adapter style bytes, or none to clear the setting.
    pub style: Option<Vec<u8>>,
}

/// Identifies an inline style reference or an immutable style revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStyleReference {
    /// The style value was supplied inline.
    Inline,
    /// The style value was resolved from an immutable semantic revision.
    Revision {
        /// Stable style entity identity.
        entity: String,
        /// Exact immutable style revision.
        revision: String,
    },
}

/// Identifies a domain-owned style schema on the wire.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSchemaReference {
    /// Stable adapter/schema identifier.
    pub adapter_id: String,
    /// Adapter-owned schema version.
    pub version: String,
}

/// Describes the generic value shape of a schema property.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStyleValueKind {
    /// Boolean value.
    Boolean,
    /// Bounded or unbounded integer.
    Integer,
    /// Value from an explicit allowed set.
    Enum,
    /// Free text value.
    Text,
    /// Opaque adapter bytes shown read-only by generic clients.
    Opaque,
}

/// Names the purposes for which one style property applies.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStyleApplicability {
    /// Applies only to personal views.
    ViewOnly,
    /// Applies only to shared materialization.
    MaterializationOnly,
    /// Applies to views and materialization.
    Both,
}

/// Describes one adapter-owned style property for generic clients.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleProperty {
    /// Stable adapter-owned property identifier.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Optional longer explanation.
    pub description: Option<String>,
    /// Generic value shape.
    pub kind: ProjectionStyleValueKind,
    /// Allowed values for enum properties.
    pub allowed_values: Vec<String>,
    /// Inclusive lower integer bound.
    pub minimum: Option<i64>,
    /// Inclusive upper integer bound.
    pub maximum: Option<i64>,
    /// Canonical default value bytes.
    pub default: Vec<u8>,
    /// Purposes this property can affect.
    pub applicability: ProjectionStyleApplicability,
    /// Whether the value can change fidelity.
    pub affects_fidelity: bool,
    /// Whether materialization must retain this property.
    pub required_for_materialization: bool,
}

/// Returns one adapter-owned style schema.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSchemaResponse {
    /// Schema identity and version.
    pub reference: ProjectionStyleSchemaReference,
    /// Projection kinds interpreted by the schema.
    pub projection_kinds: Vec<String>,
    /// Ordered property descriptors.
    pub properties: Vec<ProjectionStyleProperty>,
}

/// Reports the editability classification of one exact resolved style.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProjectionStyleFidelity {
    /// The representation preserves all modeled semantics.
    LosslessEditable,
    /// Omitted semantics can be reconciled safely with evidence.
    LossAwareEditable,
    /// Editing could destroy semantics and is disallowed.
    ReadOnlyLossy,
}

/// Reports one source that contributed to a resolved style.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSource {
    /// Stable source category label.
    pub kind: String,
    /// Stable caller-owned source identity.
    pub identity: String,
    /// Exact semantic revision when available.
    pub revision: Option<String>,
    /// Mutable source generation when available.
    pub generation: Option<u64>,
    /// Exact value contributed by this source.
    pub value: Vec<u8>,
}

/// Reports style fidelity and any omitted semantic categories.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleFidelityReport {
    /// Exact editability classification.
    pub fidelity: ProjectionStyleFidelity,
    /// Adapter-owned semantic categories omitted from the representation.
    pub omitted_categories: Vec<String>,
    /// Optional safe explanation for clients.
    pub explanation: Option<String>,
}

/// Returns an effective style with provenance and fidelity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleResolveResponse {
    /// Purpose used for resolution.
    pub purpose: ProjectionStylePurpose,
    /// Resolved style provenance reference.
    pub reference: ProjectionStyleReference,
    /// Exact canonical style bytes a renderer consumes.
    pub value: Vec<u8>,
    /// Optional schema interpreting the value.
    pub schema: Option<ProjectionStyleSchemaReference>,
    /// Ordered contributing style sources.
    pub applied: Vec<ProjectionStyleSource>,
    /// Exact resolved-style fidelity.
    pub fidelity: ProjectionStyleFidelityReport,
}

/// Returns the authoritative stored value after one setting mutation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectionStyleSettingResponse {
    /// Stored canonical style bytes, or none after clearing the setting.
    pub style: Option<Vec<u8>>,
    /// Monotonic revision, or none when the setting is absent.
    pub revision: Option<u64>,
}

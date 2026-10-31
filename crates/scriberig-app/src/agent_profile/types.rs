//! Defines provider-neutral semantic profile and context-component values.

pub use scriberig_agent_core::{
    AgentProfileDescriptor, AgentProfileExecutionPreference, AgentProfileRevisionRef,
    ContextActivationRule, ContextComponentDescriptor, ContextComponentKind,
    ContextComponentRevisionRef,
};
use scriberig_core::{EntityId, SnapshotId};

/// Requests an explicit candidate mapping for one imported legacy profile selection.
///
/// # Public members
/// - `session`: Session containing the preserved legacy choice.
/// - `expected_revision`: Session-selection revision observed during recovery inspection.
/// - `legacy_path`: Exact recovery-record source path.
/// - `base_profile_snapshot`: Exact profile-store base for the candidate.
/// - `profile`: Operator-supplied semantic profile content and stable identity.
/// - `provider_id`: Active upstream provider identity selected by the operator.
/// - `model_id`: Active upstream model identity selected by the operator.
/// - `thinking_level`: Optional upstream thinking-level identity.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyProfileMappingRequest {
    /// Session containing the preserved legacy choice.
    pub session: EntityId,
    /// Session-selection revision observed during recovery inspection.
    pub expected_revision: u64,
    /// Exact recovery-record source path.
    pub legacy_path: String,
    /// Exact profile-store base for the candidate.
    pub base_profile_snapshot: SnapshotId,
    /// Operator-supplied semantic profile content and stable identity.
    pub profile: AgentProfileDescriptor,
    /// Active upstream provider identity selected by the operator.
    pub provider_id: String,
    /// Active upstream model identity selected by the operator.
    pub model_id: String,
    /// Optional upstream thinking-level identity.
    pub thinking_level: Option<String>,
}

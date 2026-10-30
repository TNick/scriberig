//! Defines provider-neutral agent profile references and descriptors.

use crate::context_component::ContextComponentDescriptor;

/// Identifies one exact immutable agent-profile revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProfileRevisionRef {
    /// Stable profile identity independent of its display name.
    pub identity: String,
    /// Exact immutable revision identity.
    pub revision: String,
}

/// Stores a credential-free preferred upstream provider/model/thinking choice.
///
/// # Public members
/// - `provider_id`: Stable upstream provider identity.
/// - `model_id`: Stable provider-scoped model identity.
/// - `thinking_level`: Optional canonical upstream thinking-level identifier.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProfileExecutionPreference {
    /// Stable upstream provider identity.
    pub provider_id: String,
    /// Stable provider-scoped upstream model identity.
    pub model_id: String,
    /// Optional canonical upstream thinking-level identifier.
    pub thinking_level: Option<String>,
}

/// Describes a reusable profile and its ordered component definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProfileDescriptor {
    /// Stable profile identity and exact immutable revision.
    pub reference: AgentProfileRevisionRef,
    /// Human-readable profile name.
    pub name: String,
    /// Ordered provider-neutral context components used by the profile.
    pub components: Vec<ContextComponentDescriptor>,
    /// Preferred upstream selection, without catalog metadata or credentials.
    pub preferred_execution: Option<AgentProfileExecutionPreference>,
}

//! Defines reusable semantic agent-profile values.

use crate::ContextComponentDescriptor;

/// Identifies one exact immutable agent-profile revision.
///
/// # Public members
/// - `identity`: Stable profile identity.
/// - `revision`: Exact immutable profile revision.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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
/// - `thinking_level`: Optional upstream thinking-level identity.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AgentProfileExecutionPreference {
    /// Stable upstream provider identity.
    pub provider_id: String,
    /// Stable provider-scoped upstream model identity.
    pub model_id: String,
    /// Optional canonical upstream thinking-level identifier.
    pub thinking_level: Option<String>,
}

/// Describes a reusable profile and its ordered component definitions.
///
/// # Public members
/// - `reference`: Stable profile identity and exact revision.
/// - `name`: Human-readable profile name.
/// - `components`: Ordered component definitions pinned to exact revisions.
/// - `preferred_execution`: Optional credential-free upstream preference.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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

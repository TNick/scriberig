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

/// Describes a reusable profile and its ordered component definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProfileDescriptor {
    /// Stable profile identity and exact immutable revision.
    pub reference: AgentProfileRevisionRef,
    /// Human-readable profile name.
    pub name: String,
    /// Ordered provider-neutral context components used by the profile.
    pub components: Vec<ContextComponentDescriptor>,
}

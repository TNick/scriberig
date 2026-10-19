//! Advertises protocol capabilities supported by the application façade.

use super::RuntimeCapability;

/// Builds the static capability list supported by this server façade.
///
/// - `super`: Parent module containing the protocol application adapter.
///
/// Available inspect, explorer, and agent session capabilities.
/// # Returns
/// The operation result.
pub(super) fn capabilities() -> Vec<RuntimeCapability> {
    [
        ("inspect_entity", "Inspect one entity"),
        ("explorer_list", "List entities in an exact snapshot"),
        ("agent_start", "Start provider-neutral agent work"),
        ("agent_continue", "Continue work on a session branch"),
        ("agent_plan", "Create and coordinate durable agent plans"),
    ]
    .into_iter()
    .map(|(id, description)| {
        RuntimeCapability::new(
            scriberig_app::CapabilityId::new(id).expect("static capability"),
            description,
            true,
        )
    })
    .collect()
}

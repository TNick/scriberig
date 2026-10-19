//! Restricts delegated child grants to explicitly requested capabilities.

use scriberig_app::{Capability, CapabilityAction, CapabilityContext};

/// Restricts child grants to the explicitly delegated capability names.
///
/// - `parent`: Capabilities available to the parent branch.
/// - `requested`: Capability identifiers delegated by the parent model.
///
/// A child capability context that is no broader than the parent request.
/// # Returns
/// The operation result.
pub(super) fn delegated_capabilities(
    parent: &CapabilityContext,
    requested: &[String],
) -> CapabilityContext {
    let requested = requested
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let granted = parent
        .granted
        .iter()
        .filter(|capability| requested.contains(capability.as_str()))
        .cloned()
        .collect();
    let grants = parent
        .grants
        .iter()
        .filter_map(|grant| {
            let actions = grant
                .actions
                .iter()
                .filter(|action| match action {
                    CapabilityAction::Inspect => requested.contains("inspect.entity"),
                    CapabilityAction::Query => requested.contains("query"),
                    CapabilityAction::Custom(name) => requested.contains(name.as_str()),
                    _ => false,
                })
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            (!actions.is_empty()).then(|| Capability {
                actions,
                ..grant.clone()
            })
        })
        .collect();
    CapabilityContext { granted, grants }
}

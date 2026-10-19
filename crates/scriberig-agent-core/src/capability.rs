//! Filters agent tool exposure without replacing application authorization.

use scriberig_app::{ApplicationContext, CapabilityId};

/// Stable capability required by the inspect agent tool.
pub const INSPECT_CAPABILITY: &str = "inspect.entity";

/// Reports whether the trusted context exposes the inspect tool surface.
///
/// - `context`: Trusted application context.
///
/// `true` when the inspect capability is granted.
#[must_use]
/// # Returns
/// The operation result.
pub fn inspect_available(context: &ApplicationContext) -> bool {
    CapabilityId::new(INSPECT_CAPABILITY)
        .is_ok_and(|capability| context.capabilities.allows(&capability))
}

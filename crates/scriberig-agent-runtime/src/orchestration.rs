//! Holds `ScribeRig`'s semantic branch-work safety limits and ordering.

use std::sync::atomic::{AtomicU64, Ordering};

/// ScribeRig-owned execution activity sequence for one branch run.
///
/// Model-request capture and settled tool calls draw from one sequence so
/// durable activity keeps the real execution order and never adopts a
/// provider-side message index as ordering identity.
///
/// # Public members
/// - `next`: Allocate the next activity position.
///
/// # Private members
/// - `next_position`: The next unused position.
#[derive(Debug, Default)]
pub struct ActivitySequence {
    /// Next unused activity position within one branch.
    next_position: AtomicU64,
}

impl ActivitySequence {
    /// Allocates the next activity position for one branch.
    ///
    /// # Returns
    /// The next monotonic position in this branch's activity sequence.
    pub fn next(&self) -> u64 {
        self.next_position.fetch_add(1, Ordering::AcqRel)
    }
}

/// ScribeRig-owned bounds for recursive branch execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    /// Maximum semantic tool calls in one branch.
    pub max_tool_calls: usize,
    /// Maximum delegated child branches started by one branch.
    pub max_child_branches: usize,
    /// Maximum nested `ScribeRig` branch depth.
    pub max_depth: usize,
    /// Maximum simultaneously active branches in one `ScribeRig` tree.
    pub max_active_branches: usize,
}

impl Default for RuntimeLimits {
    /// Creates conservative bounded runtime limits.
    ///
    /// Default per-branch execution limits.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self {
            max_tool_calls: 64,
            max_child_branches: 16,
            max_depth: 8,
            max_active_branches: 32,
        }
    }
}

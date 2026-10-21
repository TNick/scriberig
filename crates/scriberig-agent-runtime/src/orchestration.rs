//! Holds `ScribeRig`'s semantic branch-work safety limits.

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

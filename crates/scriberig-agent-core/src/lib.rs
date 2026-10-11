//! Defines reusable agent-facing tools, capability exposure, navigation, and context interfaces.

pub mod capability;
pub mod inspect;
pub mod tool;

pub use capability::{INSPECT_CAPABILITY, inspect_available};
pub use inspect::InspectTool;
pub use tool::{Tool, ToolDetail, ToolError, ToolInput, ToolResult};

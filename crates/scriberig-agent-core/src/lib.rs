//! Defines reusable agent-facing tools, capability exposure, navigation, and context interfaces.

pub mod capability;
pub mod context_component;
pub mod inspect;
pub mod profile;
pub mod tool;

pub use capability::{INSPECT_CAPABILITY, inspect_available};
pub use context_component::{
    ContextActivationRule, ContextComponentDescriptor, ContextComponentKind,
    ContextComponentRevisionRef,
};
pub use inspect::InspectTool;
pub use profile::{AgentProfileDescriptor, AgentProfileRevisionRef};
pub use tool::{Tool, ToolDetail, ToolError, ToolInput, ToolResult};

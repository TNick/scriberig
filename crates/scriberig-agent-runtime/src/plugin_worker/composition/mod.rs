//! Converts host-authorized plugin worker tools into branch-scoped tools.
//!
//! This is the composition seam between an upstream isolated plugin worker and
//! one `ScribeRig` branch. `host::compose_branch_plugin_tools` is the only
//! entry point: it starts a worker only from an explicitly injected launcher,
//! a host tool allowlist, and a host launch policy, and never from the
//! environment. A worker tool becomes a `SemanticAgentTool` only when
//! (1) the running worker process is attested as OS-isolated, (2) the tool name
//! is both advertised by that worker and allowlisted by host policy, and (3) the
//! branch capability snapshot grants the plugin capability recorded on the
//! merged descriptor. Every settled call is reauthorized against that same
//! descriptor, bounded, and recorded as durable `ScribeRig` tool evidence.
//!
//! The launcher guarantee set that gates this seam is launcher self-report; the
//! `ScribeRig` host still has to inject a verified platform sandbox launcher, so
//! product activation stays closed until that launcher is validated.

mod bounded;
mod bridge;
mod dispatch;
mod host;

pub use bridge::{
    ComposedPluginTools, PluginCompositionError, PluginToolBridge, PluginToolRequest,
};
pub use host::{PluginWorkerAdmission, compose_branch_plugin_tools};

//! Defines ordered, provider-neutral turns and tool interactions.
use scriberig_core::EntityId;
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct HumanInput {
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct AgentOutput {
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct ToolInvocation {
    pub id: EntityId,
    pub name: String,
    pub input: Vec<u8>,
    /// Model invocation whose response requested this tool, when known.
    #[serde(default)]
    pub parent_model_invocation: Option<EntityId>,
    /// Optional schema-redacted structured arguments for safe presentation.
    #[serde(default)]
    pub presentation_input: Option<Vec<u8>>,
    /// Optional trusted compact command label for presentation.
    #[serde(default)]
    pub presentation_label: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct ToolResult {
    pub id: EntityId,
    pub invocation: EntityId,
    pub output: Vec<u8>,
    pub failed: bool,
    /// Optional bounded, allowlisted result preview for presentation.
    #[serde(default)]
    pub presentation_preview: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Turn {
    pub id: EntityId,
    pub ordinal: u64,
    pub human: Option<HumanInput>,
    pub agent: Option<AgentOutput>,
    pub tool_invocation: Option<EntityId>,
    pub tool_result: Option<EntityId>,
}

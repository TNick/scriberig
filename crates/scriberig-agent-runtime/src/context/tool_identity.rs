//! Provides stable identity encoding for trusted tool catalogs.

use rpi_ai::types::Tool;

/// Retains a tool catalog in a stable identity string.
///
/// - `tools`: Trusted tool definitions to identify.
///
/// Deterministic catalog identity text.
///
/// # Returns
/// Stable text identifying each trusted tool definition.
#[must_use]
pub fn tool_catalog_identity(tools: &[Tool]) -> String {
    tools
        .iter()
        .map(|tool| {
            format!(
                "{}:{}:{}",
                tool.name,
                tool.description,
                serde_json::to_string(&tool.parameters).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

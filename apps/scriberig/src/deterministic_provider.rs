//! Scripts `ScribeRig` onboarding behavior with upstream request and response types.

use rpi_ai::{Context, Model};

use crate::scenario_helpers::{final_response, tool_call, tool_messages, user_prompt};

#[path = "deterministic_provider/handles.rs"]
mod handles;
pub(super) use handles::parse_handles;

/// Selects real onboarding tools through the normal upstream model loop.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BootstrapProvider;

/// Returns a fixed arithmetic answer for the native conversation scenario.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ArithmeticProvider;

impl BootstrapProvider {
    #[expect(
        clippy::too_many_lines,
        reason = "The deterministic onboarding transcript is one ordered semantic scenario over upstream tool messages."
    )]
    /// Selects the next semantic onboarding operation from upstream context.
    ///
    /// # Parameters
    /// - `context`: Exact upstream provider context.
    /// - `model`: Exact upstream model bound to the scenario.
    ///
    /// # Returns
    /// An upstream final response or semantic tool call.
    ///
    /// # Errors
    /// Returns a safe explanation when expected semantic tool evidence is absent.
    pub(super) fn respond(
        context: &Context,
        model: &Model,
    ) -> Result<rpi_ai::AssistantMessage, String> {
        let prompt = user_prompt(context);
        if !prompt.to_lowercase().contains("onboard") {
            return semantic_discovery_response(context, model);
        }
        let messages = tool_messages(context);
        let response = if messages
            .iter()
            .any(|message| message.contains("active semantic workspace"))
        {
            if messages.iter().any(|message| {
                message.contains("summary=None") || message.contains("\"summary\":null")
            }) {
                return Ok(final_response(
                    model,
                    "The current project is onboarded and its semantic root was inspected.",
                ));
            }
            let workspace = messages
                .iter()
                .find(|message| message.contains("active semantic workspace"))
                .expect("workspace match exists");
            let handles = parse_handles(workspace)?;
            let snapshot = handles
                .get(1)
                .ok_or_else(|| "workspace snapshot is missing".to_owned())?;
            let root = handles
                .get(2)
                .ok_or_else(|| "workspace root is missing".to_owned())?;
            tool_call(
                model,
                "inspect",
                serde_json::json!({"entity":root,"snapshot":snapshot,"detail":"compact"}),
            )
        } else if messages.is_empty() {
            tool_call(model, "runtime_capabilities", serde_json::json!({}))
        } else if messages
            .iter()
            .any(|message| message.contains("runtime capabilities available"))
            && !messages
                .iter()
                .any(|message| message.contains("scan persisted for"))
        {
            tool_call(
                model,
                "project_scan",
                serde_json::json!({"path_ref":"current_working_directory","git_tracked":true}),
            )
        } else if messages
            .iter()
            .any(|message| message.contains("project import succeeded"))
        {
            let scan = messages
                .iter()
                .find(|message| message.contains("scan persisted for"))
                .ok_or_else(|| "project import has no scan reference".to_owned())?;
            let project = parse_handles(scan)?
                .into_iter()
                .next()
                .ok_or_else(|| "project identity is missing".to_owned())?;
            tool_call(
                model,
                "project_workspace",
                serde_json::json!({"project":project}),
            )
        } else if let Some(manifest_page) = messages.iter().rev().find(|message| {
            message.contains("summary=Some(\"manifest ")
                || message.contains("\"summary\":\"manifest ")
        }) {
            let scan = messages
                .iter()
                .find(|message| message.contains("scan persisted for"))
                .ok_or_else(|| "scan response is missing".to_owned())?;
            let handles = parse_handles(scan)?;
            let project = handles
                .first()
                .ok_or_else(|| "project identity is missing".to_owned())?;
            let manifest_id = handles
                .get(1)
                .ok_or_else(|| "manifest identity is missing".to_owned())?;
            if let Some(next_offset) = parse_handles(manifest_page)?
                .iter()
                .find_map(|handle| handle.strip_prefix("next_offset:"))
            {
                let offset = next_offset
                    .parse::<usize>()
                    .map_err(|_| "invalid manifest continuation offset".to_owned())?;
                tool_call(
                    model,
                    "project_manifest",
                    serde_json::json!({"project":project,"manifest_id":manifest_id,"offset":offset,"limit":100}),
                )
            } else {
                tool_call(
                    model,
                    "project_import",
                    serde_json::json!({"project":project,"manifest_id":manifest_id}),
                )
            }
        } else if let Some(scan) = messages
            .iter()
            .find(|message| message.contains("scan persisted for"))
        {
            let handles = parse_handles(scan)?;
            if handles.len() < 2 {
                return Err(
                    "project scan response omitted project or manifest references".to_owned(),
                );
            }
            tool_call(
                model,
                "project_manifest",
                serde_json::json!({"project":handles[0],"manifest_id":handles[1],"offset":0,"limit":100}),
            )
        } else {
            return Err(
                "onboarding tool sequence did not produce a reviewable manifest".to_owned(),
            );
        };
        Ok(response)
    }
}

/// Drives exact-snapshot semantic discovery and a read-only child branch.
///
/// # Parameters
/// - `context`: Provider context compiled for the root or child branch.
/// - `model`: Exact upstream model bound to the scenario.
///
/// # Returns
/// A semantic tool call, delegated request, or concise completion.
///
/// # Errors
/// Returns a safe explanation when the exact project scope or semantic handles are absent.
fn semantic_discovery_response(
    context: &Context,
    model: &Model,
) -> Result<rpi_ai::AssistantMessage, String> {
    let scope = project_scope(context)?;
    let messages = tool_messages(context);
    let child = user_prompt(context).contains("Find the existing code-domain");
    if child {
        if messages
            .iter()
            .any(|message| message.contains("source_digest:"))
        {
            return Ok(final_response(
                model,
                "Use the shared code domain and Tree-sitter adapter pattern; integrate classification, project dispatch, Cargo metadata, registries, and conformance fixtures. Preserve exact bytes, deterministic syntax, diagnostics, and inert parsing.",
            ));
        }
        if let Some(found) = messages
            .iter()
            .find(|message| message.contains("semantic find"))
        {
            let entity = semantic_entity(found)?;
            return Ok(tool_call(
                model,
                "semantic_render",
                serde_json::json!({"project":scope.0,"snapshot":scope.1,"entity":entity}),
            ));
        }
        return Ok(tool_call(
            model,
            "semantic_find",
            serde_json::json!({"project":scope.0,"snapshot":scope.1,"query":"lib.rs","limit":10}),
        ));
    }
    if messages
        .iter()
        .any(|message| message.contains("child_summary="))
    {
        return Ok(final_response(
            model,
            "I inspected the source structure and delegated a bounded architecture map at the same project snapshot.",
        ));
    }
    if let Some(found) = messages
        .iter()
        .find(|message| message.contains("semantic find"))
    {
        if messages
            .iter()
            .any(|message| message.contains("source_digest:"))
        {
            return Ok(tool_call(
                model,
                "delegate",
                serde_json::json!({"purpose":"determine minimal extension points for PHP source ingestion","prompt":"Find the existing code-domain and Tree-sitter language adapter patterns, then summarize the scanner, importer, Cargo, registry, and conformance integration points.","capabilities":["inspect.entity","semantic_find","semantic_render"],"expected_result":"compact architecture map with exact semantic references"}),
            ));
        }
        let entity = semantic_entity(found)?;
        return Ok(tool_call(
            model,
            "semantic_render",
            serde_json::json!({"project":scope.0,"snapshot":scope.1,"entity":entity}),
        ));
    }
    Ok(tool_call(
        model,
        "semantic_find",
        serde_json::json!({"project":scope.0,"snapshot":scope.1,"query":"lib.rs","limit":10}),
    ))
}

/// Reads exact project and snapshot handles from the upstream system prompt.
///
/// # Parameters
/// - `context`: Provider context containing `ScribeRig`-compiled semantic scope.
///
/// # Returns
/// Exact project and snapshot string identities.
///
/// # Errors
/// Returns an error when the compiled context lacks exact scope values.
fn project_scope(context: &Context) -> Result<(String, String), String> {
    let scope = context
        .system_prompt
        .as_deref()
        .and_then(|prompt| prompt.split_once("semantic_project_scope:"))
        .map(|(_, scope)| scope)
        .ok_or_else(|| "project scope is missing from compiled context".to_owned())?;
    let value = |key: &str| {
        scope
            .split_whitespace()
            .find_map(|part| part.strip_prefix(&format!("{key}=")))
            .map(str::to_owned)
            .ok_or_else(|| format!("{key} scope is missing"))
    };
    Ok((value("project")?, value("snapshot")?))
}

/// Extracts a semantic entity from a find result's stable JSON handle.
///
/// # Parameters
/// - `message`: Semantic find result text.
///
/// # Returns
/// Exact entity identity returned by semantic discovery.
///
/// # Errors
/// Returns an error when no semantic entity handle exists.
fn semantic_entity(message: &str) -> Result<String, String> {
    parse_handles(message)?
        .iter()
        .filter_map(|handle| serde_json::from_str::<serde_json::Value>(handle).ok())
        .find(|value| value.get("kind").and_then(serde_json::Value::as_str) == Some("source"))
        .and_then(|value| value.get("entity")?.as_str().map(str::to_owned))
        .ok_or_else(|| "semantic entity is missing".to_owned())
}

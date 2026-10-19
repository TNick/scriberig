//! Compiles semantic branch context into bounded provider messages.

use super::{BranchExecutionContext, CompactResultContext, ModelMessage, ToolDefinition};
use std::fmt::Write;

/// Deterministic context selection limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextPolicy {
    /// Maximum estimated characters in compiled context.
    pub max_chars: usize,
}

impl Default for ContextPolicy {
    /// Creates a bounded default context policy.
    ///
    /// The default character budget.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self { max_chars: 16_000 }
    }
}

/// Provider-neutral compiled context and exact dependency summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledContext {
    /// Ordered messages supplied to a provider adapter.
    pub messages: Vec<ModelMessage>,
    /// Exact semantic dependency identities used by compilation.
    pub dependencies: Vec<String>,
    /// Handles for details omitted by the budget.
    pub omitted_handles: Vec<String>,
    /// Canonical representation used by request identity.
    pub canonical: String,
}

/// Compiles branch context using deterministic priority and ordering rules.
pub struct ContextCompiler {
    /// Bounded selection policy.
    policy: ContextPolicy,
}

impl ContextCompiler {
    /// Creates a compiler with the supplied budget policy.
    ///
    /// - `policy`: Deterministic context selection policy.
    ///
    /// A configured context compiler.
    #[must_use]
    pub const fn new(policy: ContextPolicy) -> Self {
        Self { policy }
    }

    /// Compiles required branch inputs and bounded current-branch activity.
    ///
    /// - `context`: Explicit branch execution inputs.
    ///
    /// Canonical provider-neutral context and dependency manifest.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn compile(&self, context: &BranchExecutionContext) -> CompiledContext {
        let mut messages = Vec::new();
        let mut dependencies = vec![
            format!("session:{}", context.session),
            format!("branch:{}", context.branch),
        ];
        let mut canonical = String::new();
        if let Some(anchor) = &context.continuation_anchor {
            dependencies.push(format!("continuation-anchor:{anchor}"));
            push_message(&mut messages, &mut canonical, "continuation_anchor", anchor);
        }
        push_message(
            &mut messages,
            &mut canonical,
            "system",
            "You are operating within one isolated semantic branch.",
        );
        push_message(&mut messages, &mut canonical, "purpose", &context.purpose);
        if let Some(summary) = &context.parent_summary {
            push_message(&mut messages, &mut canonical, "parent", summary);
            dependencies.push(format!("parent-summary:{summary}"));
        }
        append_selected_results(
            &mut messages,
            &mut canonical,
            &mut dependencies,
            &context.ancestor_results,
            "ancestor_result",
            "ancestor-result",
        );
        append_selected_results(
            &mut messages,
            &mut canonical,
            &mut dependencies,
            &context.dependency_results,
            "dependency_result",
            "dependency-result",
        );
        if let Some(scope) = &context.project_scope {
            let orientation = format!(
                "project={} snapshot={} root={} selection={}",
                scope.project,
                scope.snapshot,
                scope.root,
                scope.selection.as_deref().unwrap_or("none")
            );
            push_message(
                &mut messages,
                &mut canonical,
                "semantic_project_scope",
                &orientation,
            );
            dependencies.extend([
                format!("project:{}", scope.project),
                format!("snapshot:{}", scope.snapshot),
                format!("root:{}", scope.root),
            ]);
            if let Some(selection) = &scope.selection {
                dependencies.push(format!("selection:{selection}"));
            }
        }
        push_message(
            &mut messages,
            &mut canonical,
            "instruction",
            &context.instruction,
        );
        let mut activities = context.activities.clone();
        activities.sort_by(|left, right| {
            left.ordinal
                .cmp(&right.ordinal)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.content.cmp(&right.content))
        });
        let required_chars = canonical.len();
        let mut used = required_chars;
        let mut omitted_handles = Vec::new();
        for activity in activities {
            let entry = format!("{}:{}", activity.kind, activity.content);
            if used + entry.len() > self.policy.max_chars {
                if let Some(handle) = activity.handle {
                    omitted_handles.push(handle);
                }
                continue;
            }
            used += entry.len();
            push_message(
                &mut messages,
                &mut canonical,
                &activity.kind,
                &activity.content,
            );
            dependencies.push(format!("activity:{}:{}", activity.kind, activity.ordinal));
        }
        CompiledContext {
            messages,
            dependencies,
            omitted_handles,
            canonical,
        }
    }
}

/// Adds selected durable results and their exact semantic handles.
///
/// - `messages`: Provider messages receiving compact result context.
/// - `canonical`: Canonical request identity receiving serialized content.
/// - `dependencies`: Dependency manifest receiving exact result handles.
/// - `results`: Selected durable results to include.
/// - `role`: Provider message role for this result class.
/// - `dependency_kind`: Manifest prefix identifying the result class.
fn append_selected_results(
    messages: &mut Vec<ModelMessage>,
    canonical: &mut String,
    dependencies: &mut Vec<String>,
    results: &[CompactResultContext],
    role: &str,
    dependency_kind: &str,
) {
    let mut ordered = results.to_vec();
    ordered.sort_by(|left, right| left.handle.cmp(&right.handle));
    for result in ordered {
        let rendered_result = format_result_context(&result);
        push_message(messages, canonical, role, &rendered_result);
        dependencies.push(format!("{dependency_kind}:{}", result.handle));
        dependencies.extend(
            result
                .semantic_handles
                .iter()
                .map(|handle| format!("semantic-handle:{handle}")),
        );
    }
}

/// Formats a compact durable result without activity or transcript detail.
///
/// - `result`: Selected result summary.
///
/// Stable provider-neutral result content.
/// # Returns
/// The operation result.
fn format_result_context(result: &CompactResultContext) -> String {
    let mut handles = result.semantic_handles.clone();
    handles.sort();
    format!(
        "{} revision={}: {}; handles={}",
        result.handle,
        result.revision.as_deref().unwrap_or("none"),
        result.summary,
        handles.join(",")
    )
}

/// Adds one canonical message to the compiled context.
///
/// - `messages`: Message list receiving the compiled message.
/// - `canonical`: Canonical identity text receiving the message.
/// - `role`: Semantic message role.
/// - `content`: Message content.
fn push_message(
    messages: &mut Vec<ModelMessage>,
    canonical: &mut String,
    role: &str,
    content: &str,
) {
    let message = format!("[{role}] {content}");
    messages.push(if role == "system" {
        ModelMessage::system(message)
    } else {
        ModelMessage::user(message)
    });
    let _ = writeln!(canonical, "{role}:{content}");
}

/// Retains a tool catalog in a stable identity string.
///
/// - `tools`: Trusted tool definitions to identify.
///
/// Deterministic catalog identity text.
#[must_use]
/// # Returns
/// The operation result.
pub fn tool_catalog_identity(tools: &[ToolDefinition]) -> String {
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

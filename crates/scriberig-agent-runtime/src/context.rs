//! Compiles deterministic, branch-scoped provider-neutral model context.

use std::fmt::Write;

use crate::provider::{ModelMessage, ToolDefinition};

#[path = "context/fingerprint.rs"]
mod fingerprint;
pub use fingerprint::RequestFingerprintInput;

/// One bounded semantic activity item available to a branch compiler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextActivity {
    /// Deterministic branch-local ordering value.
    pub ordinal: u64,
    /// Semantic activity category.
    pub kind: String,
    /// Human/model-readable compact content.
    pub content: String,
    /// Stable handle that can reopen omitted detail.
    pub handle: Option<String>,
}

impl ContextActivity {
    /// Creates an activity without a reopenable handle.
    ///
    /// # Parameters
    /// - `ordinal`: Stable activity order.
    /// - `kind`: Activity category.
    /// - `content`: Compact activity content.
    ///
    /// # Returns
    /// A context activity item.
    #[must_use]
    pub fn new(ordinal: u64, kind: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            ordinal,
            kind: kind.into(),
            content: content.into(),
            handle: None,
        }
    }

    /// Attaches a stable handle for omitted detail.
    ///
    /// # Parameters
    /// - `self`: Activity being configured.
    /// - `handle`: Stable semantic handle.
    ///
    /// # Returns
    /// The activity with its reopenable handle.
    #[must_use]
    pub fn with_handle(mut self, handle: impl Into<String>) -> Self {
        self.handle = Some(handle.into());
        self
    }
}

/// Explicit branch inputs consumed by context compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchExecutionContext {
    /// Stable session reference.
    pub session: String,
    /// Stable branch reference.
    pub branch: String,
    /// Branch purpose or delegated goal.
    pub purpose: String,
    /// Current human instruction.
    pub instruction: String,
    /// Compact selected parent result summary.
    pub parent_summary: Option<String>,
    /// Exact semantic project scope inherited by this branch.
    pub project_scope: Option<ProjectSemanticScope>,
    /// Current-branch semantic activities.
    pub activities: Vec<ContextActivity>,
}

/// Identifies the exact semantic project workspace available to a branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSemanticScope {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable project snapshot.
    pub snapshot: String,
    /// Semantic source root identity.
    pub root: String,
    /// Optional focused semantic entity.
    pub selection: Option<String>,
}

impl BranchExecutionContext {
    /// Creates the required branch-scoped context inputs.
    ///
    /// # Parameters
    /// - `session`: Stable session reference.
    /// - `branch`: Stable branch reference.
    /// - `purpose`: Branch purpose.
    /// - `instruction`: Current user instruction.
    ///
    /// # Returns
    /// A branch context with no inherited history.
    #[must_use]
    pub fn new(
        session: impl Into<String>,
        branch: impl Into<String>,
        purpose: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Self {
        Self {
            session: session.into(),
            branch: branch.into(),
            purpose: purpose.into(),
            instruction: instruction.into(),
            parent_summary: None,
            project_scope: None,
            activities: Vec::new(),
        }
    }

    /// Adds one explicitly selected activity to the branch context.
    ///
    /// # Parameters
    /// - `self`: Context being configured.
    /// - `activity`: Selected branch-local activity.
    ///
    /// # Returns
    /// The context with the selected activity.
    #[must_use]
    pub fn with_activity(mut self, activity: ContextActivity) -> Self {
        self.activities.push(activity);
        self
    }

    /// Adds a compact parent result summary without inheriting parent history.
    ///
    /// # Parameters
    /// - `self`: Context being configured.
    /// - `summary`: Explicitly inherited parent summary.
    ///
    /// # Returns
    /// The context with the selected parent summary.
    #[must_use]
    pub fn with_parent_summary(mut self, summary: impl Into<String>) -> Self {
        self.parent_summary = Some(summary.into());
        self
    }

    /// Binds this branch to an exact semantic project workspace.
    ///
    /// # Parameters
    /// - `self`: Context being configured.
    /// - `scope`: Exact project and snapshot references.
    ///
    /// # Returns
    /// The context with explicit semantic workspace scope.
    #[must_use]
    pub fn with_project_scope(mut self, scope: ProjectSemanticScope) -> Self {
        self.project_scope = Some(scope);
        self
    }
}

/// Deterministic context selection limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextPolicy {
    /// Maximum estimated characters in compiled context.
    pub max_chars: usize,
}

impl Default for ContextPolicy {
    /// Creates a bounded default context policy.
    ///
    /// # Returns
    /// The default character budget.
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
    /// # Parameters
    /// - `policy`: Deterministic context selection policy.
    ///
    /// # Returns
    /// A configured context compiler.
    #[must_use]
    pub const fn new(policy: ContextPolicy) -> Self {
        Self { policy }
    }

    /// Compiles required branch inputs and bounded current-branch activity.
    ///
    /// # Parameters
    /// - `context`: Explicit branch execution inputs.
    ///
    /// # Returns
    /// Canonical provider-neutral context and dependency manifest.
    #[must_use]
    pub fn compile(&self, context: &BranchExecutionContext) -> CompiledContext {
        let mut messages = Vec::new();
        let mut dependencies = vec![
            format!("session:{}", context.session),
            format!("branch:{}", context.branch),
        ];
        let mut canonical = String::new();
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

/// Adds one canonical message to the compiled context.
///
/// # Parameters
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
/// # Parameters
/// - `tools`: Trusted tool definitions to identify.
///
/// # Returns
/// Deterministic catalog identity text.
#[must_use]
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

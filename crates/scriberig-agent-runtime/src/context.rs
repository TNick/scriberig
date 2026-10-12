//! Compiles deterministic, branch-scoped provider-neutral model context.

use std::fmt::Write;

use crate::provider::{ModelMessage, ProviderCapabilities, ToolDefinition};

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
    /// Current-branch semantic activities.
    pub activities: Vec<ContextActivity>,
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

/// Inputs that contribute to exact model request identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestFingerprintInput {
    /// Fingerprint schema version.
    pub schema_version: String,
    /// Provider profile or compatibility identity.
    pub profile: String,
    /// Provider model identity.
    pub model: String,
    /// Provider capability descriptor.
    pub capabilities: ProviderCapabilities,
    /// Canonical compiled context.
    pub compiled_context: String,
    /// Canonical tool catalog identity.
    pub tools: String,
    /// Secret deliberately excluded from identity material.
    credential: Option<String>,
}

impl RequestFingerprintInput {
    /// Creates a fingerprint input with default capability metadata.
    ///
    /// # Parameters
    /// - `profile`: Provider profile identity.
    /// - `model`: Provider model identity.
    /// - `compiled_context`: Canonical context.
    /// - `tools`: Canonical tool catalog identity.
    ///
    /// # Returns
    /// Fingerprint inputs with secret-free identity material.
    #[must_use]
    pub fn new(
        profile: impl Into<String>,
        model: impl Into<String>,
        compiled_context: impl Into<String>,
        tools: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: "model-request-v1".to_owned(),
            profile: profile.into(),
            model: model.into(),
            capabilities: ProviderCapabilities::default(),
            compiled_context: compiled_context.into(),
            tools: tools.into(),
            credential: None,
        }
    }

    /// Replaces the model identity for a fingerprint comparison.
    ///
    /// # Parameters
    /// - `self`: Fingerprint input.
    /// - `model`: New model identity.
    ///
    /// # Returns
    /// Inputs using the new model identity.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Adds a secret only to prove it is excluded from identity material.
    ///
    /// # Parameters
    /// - `self`: Fingerprint input.
    /// - `credential`: Secret value never hashed or serialized.
    ///
    /// # Returns
    /// Inputs retaining the secret outside canonical material.
    #[must_use]
    pub fn with_credential(mut self, credential: impl Into<String>) -> Self {
        self.credential = Some(credential.into());
        self
    }

    /// Computes the canonical content fingerprint.
    ///
    /// # Returns
    /// Lowercase BLAKE3 digest over secret-free request identity material.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let material = format!(
            "{}\n{}\n{}\n{:?}\n{}\n{}",
            self.schema_version,
            self.profile,
            self.model,
            self.capabilities,
            self.compiled_context,
            self.tools
        );
        blake3::hash(material.as_bytes()).to_hex().to_string()
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

//! Compiles semantic branch context into bounded provider messages.

use super::{
    BranchExecutionContext, CompactResultContext, ModelMessage, ResolvedContextComponent,
    resolve_active_context_components,
};
use crate::profile::bootstrap_default_agent_profile;
use scriberig_agent_core::{
    AgentProfileDescriptor, AgentProfileRevisionRef, ContextComponentKind,
    ContextComponentRevisionRef,
};
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
    /// Exact immutable profile revision used by compilation.
    pub profile: AgentProfileRevisionRef,
    /// Active exact component revisions in compilation order.
    pub active_components: Vec<ContextComponentRevisionRef>,
    /// Canonical compiled messages, including the current instruction.
    ///
    /// Adapters that also receive the current instruction as a runtime
    /// message must use `provider_context` to avoid sending it twice.
    pub messages: Vec<ModelMessage>,
    /// Exact semantic dependency identities used by compilation.
    pub dependencies: Vec<String>,
    /// Handles for details omitted by the budget.
    pub omitted_handles: Vec<String>,
    /// Canonical representation used by request identity.
    pub canonical: String,
    /// Provider-visible compiled context without the current user instruction.
    pub provider_context: String,
}

/// Compiles branch context using deterministic priority and ordering rules.
pub struct ContextCompiler {
    /// Bounded selection policy.
    policy: ContextPolicy,
    /// Exact profile descriptor used to compile branch instructions.
    profile: AgentProfileDescriptor,
}

impl ContextCompiler {
    /// Creates a compiler with the supplied budget policy.
    ///
    /// - `policy`: Deterministic context selection policy.
    ///
    /// A configured context compiler.
    ///
    /// # Returns
    /// A compiler using the supplied policy and bootstrap profile.
    #[must_use]
    pub fn new(policy: ContextPolicy) -> Self {
        Self {
            policy,
            profile: bootstrap_default_agent_profile(),
        }
    }

    /// Replaces the bootstrap profile with an exact resolved profile.
    ///
    /// # Parameters
    /// - `self`: Context compiler being configured.
    /// - `profile`: Exact profile descriptor for the invocation.
    ///
    /// The compiler configured to use the supplied profile.
    ///
    /// # Returns
    /// The configured context compiler.
    #[must_use]
    pub fn with_profile(mut self, profile: AgentProfileDescriptor) -> Self {
        self.profile = profile;
        self
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
        let mut dependencies = Vec::new();
        let mut canonical = String::new();
        let active_components =
            self.append_required_context(context, &mut messages, &mut dependencies, &mut canonical);
        let mut provider_context = canonical.clone();
        push_message(
            &mut messages,
            &mut canonical,
            "instruction",
            &context.instruction,
        );
        let omitted_handles = self.append_activities(
            context,
            &mut messages,
            &mut dependencies,
            &mut canonical,
            &mut provider_context,
        );
        CompiledContext {
            profile: self.profile.reference.clone(),
            active_components: active_components
                .iter()
                .map(|component| component.reference.clone())
                .collect(),
            messages,
            dependencies,
            omitted_handles,
            canonical,
            provider_context,
        }
    }

    /// Appends branch scope, profile, and selected durable results.
    ///
    /// # Parameters
    /// - `context`: Explicit branch execution inputs.
    /// - `messages`: Compiled provider messages.
    /// - `dependencies`: Exact semantic dependency identities.
    /// - `canonical`: Canonical request identity text.
    ///
    /// Active profile components in deterministic activation order.
    ///
    /// # Returns
    /// Active components used to compile the required context.
    fn append_required_context(
        &self,
        context: &BranchExecutionContext,
        messages: &mut Vec<ModelMessage>,
        dependencies: &mut Vec<String>,
        canonical: &mut String,
    ) -> Vec<ResolvedContextComponent> {
        dependencies.extend([
            format!("session:{}", context.session),
            format!("branch:{}", context.branch),
        ]);
        let active_components =
            resolve_active_context_components(&self.profile, &context.branch_role);
        let profile_ref = &self.profile.reference;
        dependencies.push(format!(
            "agent-profile:{}@{}",
            profile_ref.identity, profile_ref.revision
        ));
        let _ = writeln!(
            canonical,
            "agent-profile-ref:{}@{}",
            profile_ref.identity, profile_ref.revision
        );
        append_components(messages, canonical, dependencies, &active_components);
        if let Some(anchor) = &context.continuation_anchor {
            dependencies.push(format!("continuation-anchor:{anchor}"));
            push_message(messages, canonical, "continuation_anchor", anchor);
        }
        push_message(messages, canonical, "purpose", &context.purpose);
        if let Some(summary) = &context.parent_summary {
            push_message(messages, canonical, "parent", summary);
            dependencies.push(format!("parent-summary:{summary}"));
        }
        append_selected_results(
            messages,
            canonical,
            dependencies,
            &context.ancestor_results,
            "ancestor_result",
            "ancestor-result",
        );
        append_selected_results(
            messages,
            canonical,
            dependencies,
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
            push_message(messages, canonical, "semantic_project_scope", &orientation);
            dependencies.extend([
                format!("project:{}", scope.project),
                format!("snapshot:{}", scope.snapshot),
                format!("root:{}", scope.root),
            ]);
            if let Some(selection) = &scope.selection {
                dependencies.push(format!("selection:{selection}"));
            }
        }
        active_components
    }

    /// Adds budgeted branch activity to canonical and provider context.
    ///
    /// # Parameters
    /// - `context`: Explicit branch execution inputs.
    /// - `messages`: Compiled provider messages.
    /// - `dependencies`: Exact semantic dependency identities.
    /// - `canonical`: Canonical request identity text.
    /// - `provider_context`: Provider context excluding the current instruction.
    ///
    /// Handles for activities omitted by the context budget.
    ///
    /// # Returns
    /// Handles belonging to activities that exceeded the budget.
    fn append_activities(
        &self,
        context: &BranchExecutionContext,
        messages: &mut Vec<ModelMessage>,
        dependencies: &mut Vec<String>,
        canonical: &mut String,
        provider_context: &mut String,
    ) -> Vec<String> {
        let mut activities = context.activities.clone();
        activities.sort_by(|left, right| {
            left.ordinal
                .cmp(&right.ordinal)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.content.cmp(&right.content))
        });
        let mut used = canonical.len();
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
            push_message(messages, canonical, &activity.kind, &activity.content);
            let _ = writeln!(provider_context, "{}:{}", activity.kind, activity.content);
            dependencies.push(format!("activity:{}:{}", activity.kind, activity.ordinal));
        }
        omitted_handles
    }
}

/// Adds activated profile components and exact revision provenance.
///
/// # Parameters
/// - `messages`: Provider-neutral messages receiving component content.
/// - `canonical`: Request identity receiving component refs and content.
/// - `dependencies`: Exact profile-component refs used by the request.
/// - `components`: Active components in deterministic order.
fn append_components(
    messages: &mut Vec<ModelMessage>,
    canonical: &mut String,
    dependencies: &mut Vec<String>,
    components: &[ResolvedContextComponent],
) {
    for component in components {
        let reference = &component.reference;
        dependencies.push(format!(
            "context-component:{}@{}",
            reference.identity, reference.revision
        ));
        let _ = writeln!(
            canonical,
            "context-component-ref:{}@{}",
            reference.identity, reference.revision
        );
        push_message(
            messages,
            canonical,
            "system",
            &format!(
                "[{}] {}",
                component_kind_key(component.kind),
                component.content
            ),
        );
    }
}

/// Returns the stable section name used for a typed component kind.
///
/// # Parameters
/// - `kind`: Typed semantic purpose of a component.
///
/// The stable provider-neutral section name.
/// # Returns
/// The section name for the component kind.
fn component_kind_key(kind: ContextComponentKind) -> &'static str {
    match kind {
        ContextComponentKind::CoreInvariant => "core_invariant",
        ContextComponentKind::BehaviorPolicy => "behavior_policy",
        ContextComponentKind::DomainGuidance => "domain_guidance",
        ContextComponentKind::ToolGuidance => "tool_guidance",
        ContextComponentKind::SkillTrigger => "skill_trigger",
        ContextComponentKind::ExecutionLesson => "execution_lesson",
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

//! Compiles deterministic, branch-scoped provider-neutral model context.

/// Identifies the semantic author of a compiled context message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextMessageRole {
    /// ScribeRig-authored runtime instruction.
    System,
    /// Human-authored branch instruction.
    User,
}

/// One ScribeRig-compiled context component before upstream projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelMessage {
    /// Semantic author of the context text.
    pub role: ContextMessageRole,
    /// Text compiled from `ScribeRig` branch state.
    pub content: String,
}

impl ModelMessage {
    /// Creates a human-authored context component.
    ///
    /// # Parameters
    /// - `content`: User-authored branch instruction.
    ///
    /// # Returns
    /// A user-role context message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ContextMessageRole::User,
            content: content.into(),
        }
    }

    /// Creates a ScribeRig-authored runtime instruction.
    ///
    /// # Parameters
    /// - `content`: Runtime instruction compiled from semantic state.
    ///
    /// # Returns
    /// A system-role context message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ContextMessageRole::System,
            content: content.into(),
        }
    }
}

#[path = "context/compact_result.rs"]
mod compact_result;
#[path = "context/components.rs"]
mod components;
pub(super) use compact_result::compact_branch_result;
#[path = "context/fingerprint.rs"]
mod fingerprint;
#[path = "context/tool_identity.rs"]
mod tool_identity;
pub use components::{ResolvedContextComponent, resolve_active_context_components};
pub use fingerprint::RequestFingerprintInput;
pub use tool_identity::tool_catalog_identity;

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
    /// - `ordinal`: Stable activity order.
    /// - `kind`: Activity category.
    /// - `content`: Compact activity content.
    ///
    /// A context activity item.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// - `self`: Activity being configured.
    /// - `handle`: Stable semantic handle.
    ///
    /// The activity with its reopenable handle.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// Structural role of this branch in the canonical session tree.
    pub branch_role: BranchRole,
    /// Exact durable source point used to compile inherited conversation.
    pub continuation_anchor: Option<String>,
    /// Branch purpose or delegated goal.
    pub purpose: String,
    /// Current human instruction.
    pub instruction: String,
    /// Compact selected parent result summary.
    pub parent_summary: Option<String>,
    /// Explicit compact results selected from the structural ancestor path.
    pub ancestor_results: Vec<CompactResultContext>,
    /// Fulfilled dependency results selected by the plan scheduler.
    pub dependency_results: Vec<CompactResultContext>,
    /// Exact semantic project scope inherited by this branch.
    pub project_scope: Option<ProjectSemanticScope>,
    /// Current-branch semantic activities.
    pub activities: Vec<ContextActivity>,
}

/// Identifies whether an execution branch is the root or a child in its
/// canonical `ScribeRig` session tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchRole {
    /// The branch has no structural parent.
    Root,
    /// The branch has the given structural parent branch.
    NonRoot {
        /// Stable reference to the immediate parent branch.
        parent: String,
    },
}

/// Identifies one durable result admitted to a worker's context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactResultContext {
    /// Exact durable result handle.
    pub handle: String,
    /// Immutable result revision identity when the result has versioned state.
    pub revision: Option<String>,
    /// Bounded externally meaningful result summary.
    pub summary: String,
    /// Exact semantic handles produced by the result.
    pub semantic_handles: Vec<String>,
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
    /// - `session`: Stable session reference.
    /// - `branch`: Stable branch reference.
    /// - `purpose`: Branch purpose.
    /// - `instruction`: Current user instruction.
    ///
    /// A branch context with no inherited history.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(
        session: impl Into<String>,
        branch: impl Into<String>,
        purpose: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Self {
        Self {
            session: session.into(),
            branch: branch.into(),
            branch_role: BranchRole::Root,
            continuation_anchor: None,
            purpose: purpose.into(),
            instruction: instruction.into(),
            parent_summary: None,
            ancestor_results: Vec::new(),
            dependency_results: Vec::new(),
            project_scope: None,
            activities: Vec::new(),
        }
    }

    /// Sets the structural role from the canonical session branch topology.
    ///
    /// - `self`: Context being configured.
    /// - `branch_role`: Root or child role with its actual parent reference.
    ///
    /// The context with the selected branch role.
    ///
    /// # Returns
    /// The execution context with this branch role.
    #[must_use]
    pub fn with_branch_role(mut self, branch_role: BranchRole) -> Self {
        self.branch_role = branch_role;
        self
    }

    /// Resolves a durable session anchor into branch-local context activities.
    ///
    /// Only human inputs and visible assistant outputs through the exact anchor
    /// are selected. Model traces, tool payloads, later turns, and sibling
    /// branches are not copied into the child context.
    ///
    /// - `session`: Immutable durable session snapshot.
    /// - `execution_branch`: Branch that will receive the compiled context.
    /// - `anchor`: Exact replyable source point in the session.
    /// - `purpose`: Focused branch purpose.
    /// - `instruction`: Current worker instruction.
    ///
    /// An execution context containing only the anchored conversation prefix.
    ///
    /// Returns the session validation error when the anchor is missing or
    /// non-replyable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_session_anchor(
        session: &scriberig_session::Session,
        execution_branch: scriberig_core::EntityId,
        anchor: scriberig_session::ContinuationAnchor,
        purpose: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Result<Self, scriberig_session::SessionError> {
        let resolved = session.resolve_anchor(anchor)?;
        let source = session
            .branches
            .iter()
            .find(|branch| branch.id == resolved.branch)
            .ok_or(scriberig_session::SessionError::MissingAnchorBranch)?;
        let mut context = Self::new(
            session.id.to_string(),
            execution_branch.to_string(),
            purpose,
            instruction,
        );
        context.continuation_anchor = Some(format!("{anchor:?}"));
        for turn in source.turns.iter().take(resolved.turn_count) {
            if let Some(human) = &turn.human {
                context.activities.push(
                    ContextActivity::new(
                        turn.ordinal.saturating_mul(2),
                        "human_input",
                        human.text.clone(),
                    )
                    .with_handle(format!("turn:{}:human", turn.id)),
                );
            }
            if let Some(agent) = &turn.agent {
                context.activities.push(
                    ContextActivity::new(
                        turn.ordinal.saturating_mul(2).saturating_add(1),
                        "assistant_output",
                        agent.text.clone(),
                    )
                    .with_handle(format!("turn:{}:assistant", turn.id)),
                );
            }
        }
        if let Some(result_id) = resolved.result {
            if let Some(result) = source.result.as_ref().filter(|value| value.id == result_id) {
                context.ancestor_results.push(compact_branch_result(result));
            }
        }
        Ok(context)
    }

    /// Adds one explicitly selected activity to the branch context.
    ///
    /// - `self`: Context being configured.
    /// - `activity`: Selected branch-local activity.
    ///
    /// The context with the selected activity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_activity(mut self, activity: ContextActivity) -> Self {
        self.activities.push(activity);
        self
    }

    /// Adds a compact parent result summary without inheriting parent history.
    ///
    /// - `self`: Context being configured.
    /// - `summary`: Explicitly inherited parent summary.
    ///
    /// The context with the selected parent summary.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_parent_summary(mut self, summary: impl Into<String>) -> Self {
        self.parent_summary = Some(summary.into());
        self
    }

    /// Adds an explicitly selected compact ancestor result.
    ///
    /// - `self`: Context being configured.
    /// - `result`: Durable ancestor result summary and exact semantic handles.
    ///
    /// The context with the selected ancestor result.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_ancestor_result(mut self, result: CompactResultContext) -> Self {
        self.ancestor_results.push(result);
        self
    }

    /// Adds a fulfilled plan dependency result without its branch transcript.
    ///
    /// - `self`: Context being configured.
    /// - `result`: Exact durable dependency result.
    ///
    /// The context with the selected dependency result.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_dependency_result(mut self, result: CompactResultContext) -> Self {
        self.dependency_results.push(result);
        self
    }

    /// Binds this branch to an exact semantic project workspace.
    ///
    /// - `self`: Context being configured.
    /// - `scope`: Exact project and snapshot references.
    ///
    /// The context with explicit semantic workspace scope.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_project_scope(mut self, scope: ProjectSemanticScope) -> Self {
        self.project_scope = Some(scope);
        self
    }
}

#[path = "context/compiler.rs"]
mod compiler;
pub use compiler::{CompiledContext, ContextCompiler, ContextPolicy};

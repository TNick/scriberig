//! Defines provider-neutral, revision-bound agent context components.

/// Identifies one exact immutable context-component revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextComponentRevisionRef {
    /// Stable component identity independent of display text.
    pub identity: String,
    /// Exact immutable revision identity.
    pub revision: String,
}

/// Classifies the intended semantic use of a context component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextComponentKind {
    /// Always-applicable `ScribeRig` behavioral invariant.
    CoreInvariant,
    /// General behavioral policy for an executor.
    BehaviorPolicy,
    /// Guidance for a semantic domain.
    DomainGuidance,
    /// Guidance for use of a capability or tool.
    ToolGuidance,
    /// A trigger description for an external skill.
    SkillTrigger,
    /// A validated execution lesson available for later activation.
    ExecutionLesson,
}

/// Declares the branch-role condition for activating a component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextActivationRule {
    /// Activate for every invocation using the profile.
    Always,
    /// Activate only for a root branch.
    RootOnly,
    /// Activate only for a non-root branch.
    NonRootOnly,
}

/// Describes one provider-neutral instructional context component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextComponentDescriptor {
    /// Stable identity and exact immutable revision of the component.
    pub reference: ContextComponentRevisionRef,
    /// Typed semantic purpose of this component.
    pub kind: ContextComponentKind,
    /// Provider-neutral instructional content.
    pub content: String,
    /// Explicit invocation state required to activate this component.
    pub activation: ContextActivationRule,
    /// Stable ordering priority within its profile.
    pub priority: i32,
}

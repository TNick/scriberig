//! Defines the deterministic bootstrap `ScribeRig` agent profile.

use scriberig_agent_core::{
    AgentProfileDescriptor, AgentProfileRevisionRef, ContextActivationRule,
    ContextComponentDescriptor, ContextComponentKind, ContextComponentRevisionRef,
};

/// Stable identity for the bootstrap profile.
const PROFILE_IDENTITY: &str = "scriberig.agent-profile.default";
/// Stable identity for the always-active core invariant component.
const CORE_COMPONENT_IDENTITY: &str = "scriberig.context-component.core-invariants";
/// Stable identity for non-root branch handoff guidance.
const HANDOFF_COMPONENT_IDENTITY: &str = "scriberig.context-component.non-root-handoff";

/// Builds the versioned `ScribeRig` bootstrap profile with core invariants and child handoff.
///
/// Component and profile revisions are content-derived so identical definitions
/// resolve to the same references across processes and restarts.
///
/// The deterministic bootstrap profile descriptor.
/// # Returns
/// A profile containing the core invariant and non-root handoff components.
#[must_use]
pub fn bootstrap_default_agent_profile() -> AgentProfileDescriptor {
    let core_content = concat!(
        "Operate inside the current ScribeRig branch and context frame. ",
        "For understood domains, prefer semantic/domain operations; treat ",
        "files as projections. Make canonical changes through candidate, ",
        "migration, validation, and promotion. Use stable semantic handles ",
        "and exact snapshots/revisions when exact state matters. Use child ",
        "branches for focused context isolation; a branch alone does not imply ",
        "another agent. Parents and dependencies consume compact durable ",
        "results, not child transcripts. Use capabilities progressively."
    );
    let handoff_content = concat!(
        "When finishing this non-root branch, make the final response a compact ",
        "one-paragraph summary for the parent. Include the important outcome, ",
        "decisions or discoveries, remaining issues, and durable semantic or ",
        "result handles. Do not reproduce the branch transcript or hidden ",
        "reasoning."
    );
    let components = vec![
        component(
            CORE_COMPONENT_IDENTITY,
            ContextComponentKind::CoreInvariant,
            core_content,
            ContextActivationRule::Always,
            0,
        ),
        component(
            HANDOFF_COMPONENT_IDENTITY,
            ContextComponentKind::BehaviorPolicy,
            handoff_content,
            ContextActivationRule::NonRootOnly,
            1,
        ),
    ];
    AgentProfileDescriptor {
        reference: profile_reference(&components),
        name: "Default ScribeRig agent profile".to_owned(),
        components,
    }
}

/// Builds the profile revision from its identity and ordered component refs.
///
/// - `components`: Components in the profile's declared order.
///
/// Stable profile identity and composition revision.
/// # Returns
/// Stable identity and composition-derived revision.
fn profile_reference(components: &[ContextComponentDescriptor]) -> AgentProfileRevisionRef {
    let mut fingerprint = Fingerprint::new("scriberig-agent-profile-v1");
    fingerprint.field(PROFILE_IDENTITY);
    fingerprint.field("Default ScribeRig agent profile");
    for component in components {
        fingerprint.field(&component.reference.identity);
        fingerprint.field(&component.reference.revision);
    }
    AgentProfileRevisionRef {
        identity: PROFILE_IDENTITY.to_owned(),
        revision: fingerprint.finish(),
    }
}

/// Builds one bootstrap component with a revision derived from its definition.
///
/// - `identity`: Stable identity independent of instructional content.
/// - `kind`: Typed component purpose.
/// - `content`: Provider-neutral instructional text.
/// - `activation`: Explicit branch-role activation rule.
/// - `priority`: Stable ordering priority within the profile.
///
/// The component descriptor with a deterministic content-derived revision.
/// # Returns
/// A descriptor containing the exact content-derived component revision.
fn component(
    identity: &str,
    kind: ContextComponentKind,
    content: &str,
    activation: ContextActivationRule,
    priority: i32,
) -> ContextComponentDescriptor {
    let mut fingerprint = Fingerprint::new("scriberig-context-component-v1");
    fingerprint.field(identity);
    fingerprint.field(kind_key(kind));
    fingerprint.field(content);
    fingerprint.field(activation_key(activation));
    fingerprint.field(&priority.to_string());
    ContextComponentDescriptor {
        reference: ContextComponentRevisionRef {
            identity: identity.to_owned(),
            revision: fingerprint.finish(),
        },
        kind,
        content: content.to_owned(),
        activation,
        priority,
    }
}

/// Returns the stable key used to fingerprint a component kind.
///
/// - `kind`: Typed component purpose.
///
/// The version-stable component kind key.
/// # Returns
/// A stable key for the component kind.
fn kind_key(kind: ContextComponentKind) -> &'static str {
    match kind {
        ContextComponentKind::CoreInvariant => "core-invariant",
        ContextComponentKind::BehaviorPolicy => "behavior-policy",
        ContextComponentKind::DomainGuidance => "domain-guidance",
        ContextComponentKind::ToolGuidance => "tool-guidance",
        ContextComponentKind::SkillTrigger => "skill-trigger",
        ContextComponentKind::ExecutionLesson => "execution-lesson",
    }
}

/// Returns the stable key used to fingerprint an activation rule.
///
/// - `activation`: Explicit branch-role activation rule.
///
/// The version-stable activation key.
/// # Returns
/// A stable key for the activation rule.
fn activation_key(activation: ContextActivationRule) -> &'static str {
    match activation {
        ContextActivationRule::Always => "always",
        ContextActivationRule::RootOnly => "root-only",
        ContextActivationRule::NonRootOnly => "non-root-only",
    }
}

/// Builds an unambiguous, domain-separated content fingerprint.
struct Fingerprint {
    /// Accumulated hash state.
    hasher: blake3::Hasher,
}

impl Fingerprint {
    /// Starts a fingerprint for one versioned descriptor domain.
    ///
    /// - `domain`: Stable domain/version label.
    ///
    /// The initialized fingerprint builder.
    /// # Returns
    /// A fingerprint builder initialized for the given domain.
    fn new(domain: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        Self::write_field(&mut hasher, domain);
        Self { hasher }
    }

    /// Adds one length-delimited field to the fingerprint.
    ///
    /// - self: Fingerprint being extended.
    /// - `value`: Field value.
    fn field(&mut self, value: &str) {
        Self::write_field(&mut self.hasher, value);
    }

    /// Encodes one field without ambiguity from concatenated values.
    ///
    /// - `hasher`: Fingerprint state.
    /// - `value`: Field value.
    fn write_field(hasher: &mut blake3::Hasher, value: &str) {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }

    /// Finishes the fingerprint as a stable lowercase hexadecimal string.
    ///
    /// - self: Fingerprint being finalized.
    ///
    /// A versioned fingerprint string.
    /// # Returns
    /// The BLAKE3 digest with its schema/version prefix.
    fn finish(self) -> String {
        format!("blake3-v1:{}", self.hasher.finalize().to_hex())
    }
}

#[cfg(test)]
mod tests {
    //! Verifies stable, content-sensitive bootstrap profile references.

    use super::*;

    /// Verifies identical bootstrap definitions produce identical references.
    #[test]
    fn bootstrap_profile_is_deterministic_and_separates_handoff() {
        let first = bootstrap_default_agent_profile();
        let second = bootstrap_default_agent_profile();

        assert_eq!(first, second);
        assert_eq!(first.components.len(), 2);
        assert_eq!(
            first.components[0].activation,
            ContextActivationRule::Always
        );
        assert_eq!(
            first.components[1].activation,
            ContextActivationRule::NonRootOnly
        );
    }

    /// Verifies a behaviorally changed component creates new component/profile revisions.
    #[test]
    fn changed_component_content_changes_component_and_profile_revision() {
        let profile = bootstrap_default_agent_profile();
        let original = &profile.components[0];
        let changed = component(
            &original.reference.identity,
            original.kind,
            "changed invariant",
            original.activation,
            original.priority,
        );
        let mut changed_components = profile.components.clone();
        changed_components[0] = changed.clone();

        assert_eq!(changed.reference.identity, original.reference.identity);
        assert_ne!(changed.reference.revision, original.reference.revision);
        assert_ne!(
            profile_reference(&changed_components).revision,
            profile.reference.revision
        );
    }

    /// Verifies profile revisions preserve the declared component order.
    #[test]
    fn reordered_components_change_profile_revision_only() {
        let profile = bootstrap_default_agent_profile();
        let mut reordered_components = profile.components.clone();
        reordered_components.reverse();

        assert_eq!(
            reordered_components[0].reference,
            profile.components[1].reference
        );
        assert_eq!(
            reordered_components[1].reference,
            profile.components[0].reference
        );
        assert_ne!(
            profile_reference(&reordered_components).revision,
            profile.reference.revision
        );
    }
}

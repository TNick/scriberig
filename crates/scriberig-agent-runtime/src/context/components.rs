//! Resolves profile components against explicit branch-role state.

use scriberig_agent_core::{
    AgentProfileDescriptor, ContextActivationRule, ContextComponentKind,
    ContextComponentRevisionRef,
};

use super::BranchRole;

/// One profile component activated for an exact branch role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedContextComponent {
    /// Stable component identity and exact revision.
    pub reference: ContextComponentRevisionRef,
    /// Typed semantic purpose of this component.
    pub kind: ContextComponentKind,
    /// Provider-neutral instructional content.
    pub content: String,
    /// Stable ordering priority from the profile definition.
    pub priority: i32,
}

/// Resolves and deterministically orders active components for one branch.
///
/// # Parameters
/// - `profile`: Exact profile descriptor resolved for this invocation.
/// - `branch_role`: Explicit structural role from the `ScribeRig` session tree.
///
/// Active components ordered by priority, identity, and exact revision.
/// # Returns
/// The ordered active component list. Components failing activation are absent.
#[must_use]
pub fn resolve_active_context_components(
    profile: &AgentProfileDescriptor,
    branch_role: &BranchRole,
) -> Vec<ResolvedContextComponent> {
    let mut active = profile
        .components
        .iter()
        .filter(|component| match component.activation {
            ContextActivationRule::Always => true,
            ContextActivationRule::RootOnly => matches!(branch_role, BranchRole::Root),
            ContextActivationRule::NonRootOnly => {
                matches!(branch_role, BranchRole::NonRoot { .. })
            }
        })
        .map(|component| ResolvedContextComponent {
            reference: component.reference.clone(),
            kind: component.kind,
            content: component.content.clone(),
            priority: component.priority,
        })
        .collect::<Vec<_>>();
    active.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.reference.identity.cmp(&right.reference.identity))
            .then_with(|| left.reference.revision.cmp(&right.reference.revision))
    });
    active
}

#[cfg(test)]
mod tests {
    //! Verifies branch-role activation and deterministic component ordering.

    use super::*;
    use crate::profile::bootstrap_default_agent_profile;

    /// Verifies role activation excludes inactive components and is repeatable.
    #[test]
    fn activation_is_role_specific_and_deterministic() {
        let mut profile = bootstrap_default_agent_profile();
        let mut root_only = profile.components[0].clone();
        root_only.reference.identity = "scriberig.test.root-only".to_owned();
        root_only.reference.revision = "revision-1".to_owned();
        root_only.kind = scriberig_agent_core::ContextComponentKind::DomainGuidance;
        root_only.content = "root-only fixture".to_owned();
        root_only.activation = ContextActivationRule::RootOnly;
        profile.components.push(root_only);
        profile.components.reverse();
        for component in &mut profile.components {
            component.priority = 4;
        }

        let root = resolve_active_context_components(&profile, &BranchRole::Root);
        let child_role = BranchRole::NonRoot {
            parent: "root-branch".to_owned(),
        };
        let child = resolve_active_context_components(&profile, &child_role);

        assert_eq!(
            root,
            resolve_active_context_components(&profile, &BranchRole::Root)
        );
        assert_eq!(
            child,
            resolve_active_context_components(&profile, &child_role)
        );
        assert_eq!(root.len(), 2);
        assert_eq!(child.len(), 2);
        assert!(
            root.iter()
                .any(|component| component.reference.identity == "scriberig.test.root-only")
        );
        assert!(
            !child
                .iter()
                .any(|component| component.reference.identity == "scriberig.test.root-only")
        );
        assert!(root.iter().any(|component| {
            component.kind == scriberig_agent_core::ContextComponentKind::CoreInvariant
        }));
        assert!(child.iter().any(|component| {
            component.kind == scriberig_agent_core::ContextComponentKind::CoreInvariant
        }));
        assert!(
            root.windows(2)
                .all(|pair| pair[0].reference.identity <= pair[1].reference.identity)
        );
        assert!(
            child
                .windows(2)
                .all(|pair| pair[0].reference.identity <= pair[1].reference.identity)
        );
    }
}

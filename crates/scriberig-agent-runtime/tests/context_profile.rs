//! Verifies bootstrap profile semantics and component-bound context identity.

use scriberig_agent_core::{ContextActivationRule, ContextComponentKind};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, BranchRole, ContextCompiler, ContextPolicy,
};
use scriberig_agent_runtime::profile::bootstrap_default_agent_profile;

/// Checks the bootstrap guidance meaning and its root/non-root activation.
#[test]
fn bootstrap_guidance_semantics_follow_branch_role() {
    let profile = bootstrap_default_agent_profile();
    let core = profile
        .components
        .iter()
        .find(|component| component.kind == ContextComponentKind::CoreInvariant)
        .expect("core invariant component");
    let handoff = profile
        .components
        .iter()
        .find(|component| component.activation == ContextActivationRule::NonRootOnly)
        .expect("non-root handoff component");

    for requirement in [
        "current ScribeRig branch and context frame",
        "semantic/domain operations",
        "files as projections",
        "candidate, migration, validation, and promotion",
        "stable semantic handles and exact snapshots/revisions",
        "focused context isolation",
        "compact durable results, not child transcripts",
        "capabilities progressively",
    ] {
        assert!(core.content.contains(requirement), "missing {requirement}");
    }
    for requirement in [
        "one-paragraph summary for the parent",
        "important outcome",
        "decisions or discoveries",
        "remaining issues",
        "durable semantic or result handles",
        "Do not reproduce the branch transcript or hidden reasoning",
    ] {
        assert!(
            handoff.content.contains(requirement),
            "missing {requirement}"
        );
    }

    let root = BranchExecutionContext::new("session", "root", "goal", "start");
    let child = BranchExecutionContext::new("session", "child", "goal", "start").with_branch_role(
        BranchRole::NonRoot {
            parent: "root".to_owned(),
        },
    );
    let compiler = ContextCompiler::new(ContextPolicy::default()).with_profile(profile.clone());
    let root_context = compiler.compile(&root);
    let child_context = compiler.compile(&child);

    assert!(root_context.canonical.contains(&core.content));
    assert!(!root_context.canonical.contains(&handoff.content));
    assert!(child_context.canonical.contains(&core.content));
    assert!(child_context.canonical.contains(&handoff.content));
}

/// Checks that a component revision alone changes canonical context identity.
#[test]
fn canonical_context_changes_for_component_revision_only() {
    let profile = bootstrap_default_agent_profile();
    let core_index = profile
        .components
        .iter()
        .position(|component| component.kind == ContextComponentKind::CoreInvariant)
        .expect("core invariant component");
    let context = BranchExecutionContext::new("session", "root", "goal", "start");
    let original = ContextCompiler::new(ContextPolicy::default())
        .with_profile(profile.clone())
        .compile(&context);
    let mut changed_profile = profile.clone();
    changed_profile.components[core_index].reference.revision =
        "component-revision-only-change".to_owned();
    assert_eq!(changed_profile.reference, profile.reference);
    assert_eq!(
        changed_profile.components[core_index].content,
        profile.components[core_index].content
    );
    let changed = ContextCompiler::new(ContextPolicy::default())
        .with_profile(changed_profile)
        .compile(&context);

    assert_ne!(original.canonical, changed.canonical);
    assert!(changed.canonical.contains("component-revision-only-change"));
}

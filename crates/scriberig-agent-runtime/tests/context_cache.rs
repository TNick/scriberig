//! Verifies deterministic branch context and exact model-result caching.

use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, BranchRole, CompactResultContext, ContextActivity, ContextCompiler,
    ContextPolicy, ProjectSemanticScope, RequestFingerprintInput,
};
use scriberig_agent_runtime::plan_context::PlanWorkerContext;
use scriberig_agent_runtime::profile::bootstrap_default_agent_profile;
use scriberig_agent_runtime::provider::{FinishReason, ModelResponse, ProviderMetadata};
use scriberig_core::EntityId;
use scriberig_session::{
    AgentOutput, Branch, BranchResult, BranchStatus, ContinuationAnchor, HumanInput, Plan,
    PlanDependency, PlanNode, PlanNodeParent, PlanNodeStatus, PlanStatus, Session, Turn,
    WorkerClaim,
};

/// Verifies branch roles default to root and retain the actual parent reference.
#[test]
fn branch_execution_context_records_structural_role() {
    let root = BranchExecutionContext::new("session", "root", "goal", "start");
    assert_eq!(root.branch_role, BranchRole::Root);

    let child = BranchExecutionContext::new("session", "child", "goal", "start").with_branch_role(
        BranchRole::NonRoot {
            parent: "parent-branch".to_owned(),
        },
    );
    assert_eq!(
        child.branch_role,
        BranchRole::NonRoot {
            parent: "parent-branch".to_owned()
        }
    );
    assert_eq!(
        format!("{:?}", child.branch_role),
        "NonRoot { parent: \"parent-branch\" }"
    );
}

/// Verifies context compilation includes only role-active profile instructions.
#[test]
fn compiler_activates_components_and_records_exact_references() {
    let profile = bootstrap_default_agent_profile();
    let root = BranchExecutionContext::new("session", "root", "goal", "start");
    let child = BranchExecutionContext::new("session", "child", "goal", "start").with_branch_role(
        BranchRole::NonRoot {
            parent: "root".to_owned(),
        },
    );
    let compiler = ContextCompiler::new(ContextPolicy::default()).with_profile(profile.clone());
    let root_compiled = compiler.compile(&root);
    let child_compiled = compiler.compile(&child);
    let core = &profile.components[0];
    let handoff = &profile.components[1];

    assert!(root_compiled.canonical.contains(&core.content));
    assert!(!root_compiled.canonical.contains(&handoff.content));
    assert!(child_compiled.canonical.contains(&core.content));
    assert!(child_compiled.canonical.contains(&handoff.content));
    assert!(child_compiled.messages[0].content.ends_with(&core.content));
    assert!(
        child_compiled.messages[1]
            .content
            .ends_with(&handoff.content)
    );
    assert!(
        child_compiled.messages[2]
            .content
            .contains("[purpose] goal")
    );
    assert!(root_compiled.canonical.contains(&format!(
        "context-component-ref:{}@{}",
        core.reference.identity, core.reference.revision
    )));
    assert!(root_compiled.dependencies.contains(&format!(
        "context-component:{}@{}",
        core.reference.identity, core.reference.revision
    )));
    assert!(!root_compiled.dependencies.contains(&format!(
        "context-component:{}@{}",
        handoff.reference.identity, handoff.reference.revision
    )));
    assert!(child_compiled.dependencies.contains(&format!(
        "context-component:{}@{}",
        handoff.reference.identity, handoff.reference.revision
    )));
}

/// Keeps current instruction in canonical identity but out of the provider context.
#[test]
fn compiler_separates_current_instruction_from_provider_context() {
    let instruction = "current user instruction";
    let context = BranchExecutionContext::new("session", "root", "goal", instruction);
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&context);

    assert!(compiled.canonical.contains(instruction));
    assert!(!compiled.provider_context.contains(instruction));
}

/// Verifies exact active profile/component revisions affect canonical identity.
#[test]
fn compiler_canonical_changes_when_active_component_revision_changes() {
    let original_profile = bootstrap_default_agent_profile();
    let context = BranchExecutionContext::new("session", "root", "goal", "start");
    let original = ContextCompiler::new(ContextPolicy::default())
        .with_profile(original_profile.clone())
        .compile(&context);
    let mut changed_profile = original_profile;
    changed_profile.reference.revision = "profile-revision-2".to_owned();
    changed_profile.components[0].reference.revision = "component-revision-2".to_owned();
    let changed = ContextCompiler::new(ContextPolicy::default())
        .with_profile(changed_profile)
        .compile(&context);

    assert_ne!(original.canonical, changed.canonical);
    assert!(changed.canonical.contains("profile-revision-2"));
    assert!(changed.canonical.contains("component-revision-2"));
}

/// Verifies unrelated activity does not enter a bounded child context.
#[test]
fn child_context_is_bounded_and_preserves_omitted_handles() {
    let context = BranchExecutionContext::new("session", "child", "child purpose", "do work")
        .with_parent_summary("parent summary")
        .with_activity(ContextActivity::new(1, "old", "secret marker").with_handle("h-old"))
        .with_activity(ContextActivity::new(2, "recent", "selected marker").with_handle("h-new"));
    let compiled = ContextCompiler::new(ContextPolicy { max_chars: 80 }).compile(&context);

    assert!(compiled.canonical.contains("child purpose"));
    assert!(!compiled.canonical.contains("secret marker"));
    assert!(compiled.omitted_handles.contains(&"h-old".to_owned()));
}

/// Verifies dependency and ancestor context carries results without transcripts.
#[test]
fn worker_context_receives_only_selected_compact_results() {
    let context = BranchExecutionContext::new(
        "session",
        "worker-branch",
        "implement consumer",
        "use parser result",
    )
    .with_ancestor_result(CompactResultContext {
        handle: "result:ancestor".to_owned(),
        revision: None,
        summary: "selected root decision".to_owned(),
        semantic_handles: vec!["entity:decision".to_owned()],
    })
    .with_dependency_result(CompactResultContext {
        handle: "result:parser".to_owned(),
        revision: Some("snapshot:parser-v1".to_owned()),
        summary: "parser API is ready".to_owned(),
        semantic_handles: vec!["entity:parse".to_owned()],
    })
    .with_activity(ContextActivity::new(1, "local", "worker-local history"));
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&context);

    assert!(compiled.canonical.contains("result:parser"));
    assert!(compiled.canonical.contains("entity:parse"));
    assert!(compiled.canonical.contains("result:ancestor"));
    assert!(compiled.canonical.contains("worker-local history"));
    assert!(!compiled.canonical.contains("sibling transcript"));
    assert!(!compiled.canonical.contains("parent raw transcript"));
    assert!(
        compiled
            .dependencies
            .contains(&"dependency-result:result:parser".to_owned())
    );
    assert!(
        compiled
            .dependencies
            .contains(&"ancestor-result:result:ancestor".to_owned())
    );
    assert!(compiled.canonical.contains("snapshot:parser-v1"));
}

/// Verifies exact result revisions affect cache identity while presentation state does not.
#[test]
fn context_fingerprint_tracks_dependency_revision_only() {
    let base = BranchExecutionContext::new(
        "session",
        "worker",
        "consume result",
        "implement from dependency",
    )
    .with_dependency_result(CompactResultContext {
        handle: "result:api".to_owned(),
        revision: Some("revision:one".to_owned()),
        summary: "API is ready".to_owned(),
        semantic_handles: vec!["entity:api".to_owned()],
    });
    let first = ContextCompiler::new(ContextPolicy::default()).compile(&base);
    let repeat = ContextCompiler::new(ContextPolicy::default()).compile(&base);
    let changed = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new(
            "session",
            "worker",
            "consume result",
            "implement from dependency",
        )
        .with_dependency_result(CompactResultContext {
            handle: "result:api".to_owned(),
            revision: Some("revision:two".to_owned()),
            summary: "API is ready".to_owned(),
            semantic_handles: vec!["entity:api".to_owned()],
        }),
    );
    let first_fingerprint =
        RequestFingerprintInput::new("profile", "model", first.canonical, "tools").fingerprint();
    let repeat_fingerprint =
        RequestFingerprintInput::new("profile", "model", repeat.canonical, "tools").fingerprint();
    let changed_fingerprint =
        RequestFingerprintInput::new("profile", "model", changed.canonical, "tools").fingerprint();

    assert_eq!(first_fingerprint, repeat_fingerprint);
    assert_ne!(first_fingerprint, changed_fingerprint);
}

/// Verifies historical context stops at the exact durable reply anchor.
#[test]
fn exact_anchor_context_excludes_later_turns_and_sibling_branches() {
    let branch_id = EntityId::from_bytes([201; 16]);
    let sibling_id = EntityId::from_bytes([202; 16]);
    let first_turn = EntityId::from_bytes([203; 16]);
    let later_turn = EntityId::from_bytes([204; 16]);
    let mut branch = Branch::new(branch_id, "root").expect("root branch");
    branch.turns = vec![
        Turn {
            id: first_turn,
            ordinal: 1,
            human: Some(HumanInput {
                text: "first question".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "first answer".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
        Turn {
            id: later_turn,
            ordinal: 2,
            human: Some(HumanInput {
                text: "later question must stay out".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "later answer must stay out".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
    ];
    let mut sibling = Branch::new(sibling_id, "sibling").expect("sibling branch");
    sibling.turns.push(Turn {
        id: EntityId::from_bytes([205; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "sibling secret".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let session = Session {
        id: EntityId::from_bytes([206; 16]),
        branches: vec![branch, sibling],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let anchor = ContinuationAnchor::AssistantOutput {
        branch: branch_id,
        turn: first_turn,
    };
    let context = BranchExecutionContext::from_session_anchor(
        &session,
        EntityId::from_bytes([207; 16]),
        anchor,
        "historical continuation",
        "continue from here",
    )
    .expect("resolve historical anchor");
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&context);

    assert!(compiled.canonical.contains("first question"));
    assert!(compiled.canonical.contains("first answer"));
    assert!(!compiled.canonical.contains("later question"));
    assert!(!compiled.canonical.contains("later answer"));
    assert!(!compiled.canonical.contains("sibling secret"));
    assert!(compiled.canonical.contains("continuation_anchor"));
}

#[path = "context_cache_parts/plan_worker.rs"]
mod plan_worker;

#[path = "context_cache_parts/part1.rs"]
mod part1;

//! Verifies deterministic branch context and exact model-result caching.

use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, CompactResultContext, ContextActivity, ContextCompiler, ContextPolicy,
    ProjectSemanticScope, RequestFingerprintInput,
};
use scriberig_agent_runtime::plan_context::PlanWorkerContext;
use scriberig_agent_runtime::provider::{FinishReason, ModelResponse, ProviderMetadata};
use scriberig_core::EntityId;
use scriberig_session::{
    AgentOutput, Branch, BranchResult, BranchStatus, ContinuationAnchor, HumanInput, Plan,
    PlanDependency, PlanNode, PlanNodeParent, PlanNodeStatus, PlanStatus, Session, Turn,
    WorkerClaim,
};

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

/// Verifies plan workers receive exact anchor and result context without raw
/// sibling or prerequisite transcripts.
#[test]
#[allow(clippy::too_many_lines)] // The scenario compares all inherited and excluded context.
fn plan_worker_context_isolated_to_anchor_and_exact_results() {
    let root_id = EntityId::from_bytes([211; 16]);
    let prerequisite_branch_id = EntityId::from_bytes([212; 16]);
    let sibling_id = EntityId::from_bytes([213; 16]);
    let worker_branch_id = EntityId::from_bytes([214; 16]);
    let prerequisite_id = EntityId::from_bytes([215; 16]);
    let worker_id = EntityId::from_bytes([216; 16]);
    let result_id = EntityId::from_bytes([217; 16]);
    let session_id = EntityId::from_bytes([218; 16]);
    let plan_id = EntityId::from_bytes([219; 16]);
    let anchor_turn_id = EntityId::from_bytes([220; 16]);
    let late_turn_id = EntityId::from_bytes([221; 16]);

    let mut root = Branch::new(root_id, "root").expect("root branch");
    root.turns = vec![
        Turn {
            id: anchor_turn_id,
            ordinal: 1,
            human: Some(HumanInput {
                text: "selected anchor question".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "selected anchor answer".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
        Turn {
            id: late_turn_id,
            ordinal: 2,
            human: Some(HumanInput {
                text: "later root transcript excluded".to_owned(),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        },
    ];
    let mut prerequisite =
        Branch::new(prerequisite_branch_id, "prerequisite").expect("prerequisite branch");
    prerequisite.turns.push(Turn {
        id: EntityId::from_bytes([222; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "PRIVATE_DEPENDENCY_TRANSCRIPT".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    prerequisite.result = Some(BranchResult {
        id: result_id,
        branch: prerequisite_branch_id,
        status: BranchStatus::Completed,
        summary: Some("public compact dependency result".to_owned()),
        result_kind: Some("api-contract".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["entity:api-contract".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let mut sibling = Branch::new(sibling_id, "sibling").expect("sibling branch");
    sibling.turns.push(Turn {
        id: EntityId::from_bytes([223; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "PRIVATE_SIBLING_TRANSCRIPT".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let mut worker = Branch::new(worker_branch_id, "worker").expect("worker branch");
    worker.turns.push(Turn {
        id: EntityId::from_bytes([224; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "worker local history".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let anchor = ContinuationAnchor::AssistantOutput {
        branch: root_id,
        turn: anchor_turn_id,
    };
    let session = Session {
        id: session_id,
        branches: vec![root, prerequisite, sibling, worker],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let plan = Plan {
        id: plan_id,
        session: Some(session_id),
        anchor: Some(anchor),
        goal: "implement consumer".to_owned(),
        revision: 1,
        status: PlanStatus::Running,
        root_branch: root_id,
        nodes: vec![
            PlanNode {
                id: prerequisite_id,
                parent: Some(PlanNodeParent::Branch(root_id)),
                title: "publish API".to_owned(),
                purpose: "define API".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                status: PlanNodeStatus::Completed,
                branch: Some(prerequisite_branch_id),
                result: Some(result_id),
                superseded_by: None,
                assignment: None,
                claim: None,
                claim_generation: 1,
            },
            PlanNode {
                id: worker_id,
                parent: Some(PlanNodeParent::Branch(root_id)),
                title: "implement consumer".to_owned(),
                purpose: "use the published API".to_owned(),
                expected_result: None,
                semantic_scope: vec!["entity:consumer-module".to_owned()],
                status: PlanNodeStatus::Running,
                branch: Some(worker_branch_id),
                result: None,
                superseded_by: None,
                assignment: None,
                claim: Some(WorkerClaim {
                    id: EntityId::from_bytes([225; 16]),
                    node: worker_id,
                    branch: worker_branch_id,
                    owner: "test-worker".to_owned(),
                    generation: 1,
                    assignment: None,
                }),
                claim_generation: 1,
            },
        ],
        dependencies: vec![PlanDependency {
            prerequisite: prerequisite_id,
            dependent: worker_id,
            fulfilled_by: Some(result_id),
        }],
        coordinator: None,
        coordinator_cursor: Some(worker_id),
        coordinator_claim: None,
        coordinator_generation: 0,
        root_nodes: vec![prerequisite_id, worker_id],
        history: Vec::new(),
    };
    let claim = plan.nodes[1].claim.as_ref().expect("worker claim");

    let context = PlanWorkerContext::from_plan(&session, &plan, &plan.nodes[1], claim)
        .expect("compile exact worker context");
    let compiled = context.compiled.canonical;

    assert!(compiled.contains("selected anchor question"));
    assert!(compiled.contains("selected anchor answer"));
    assert!(compiled.contains("public compact dependency result"));
    assert!(compiled.contains("entity:api-contract"));
    assert!(compiled.contains("entity:consumer-module"));
    assert!(compiled.contains("worker local history"));
    assert!(!compiled.contains("later root transcript excluded"));
    assert!(!compiled.contains("PRIVATE_DEPENDENCY_TRANSCRIPT"));
    assert!(!compiled.contains("PRIVATE_SIBLING_TRANSCRIPT"));
}

#[path = "context_cache_parts/part1.rs"]
mod part1;

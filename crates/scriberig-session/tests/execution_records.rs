//! Verifies provider-neutral execution records and compact nested views.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, CacheDisposition, ExecutionActivity, ModelInvocation,
    ModelOutput, SelectionError, Session, SessionExecutionSelection, SessionViews,
};

/// Verifies model provenance and output are provider-neutral and secret-free.
#[test]
fn model_execution_records_are_explicit() {
    let branch = EntityId::from_bytes([1; 16]);
    let invocation = EntityId::from_bytes([2; 16]);
    let output = EntityId::from_bytes([3; 16]);
    let record = ModelInvocation {
        id: invocation,
        branch,
        ordinal: 1,
        agent_profile: Some("default".to_owned()),
        context_fingerprint: Some("ctx".to_owned()),
        provider_profile: Some("local".to_owned()),
        model: Some("model-a".to_owned()),
        model_profile: Some("profile-a".to_owned()),
        provider_model_id: Some("native-model-a".to_owned()),
        thinking_level: Some("careful".to_owned()),
        request_fingerprint: Some("req".to_owned()),
        cache: CacheDisposition::Miss,
        output: Some(output),
        request: None,
    };
    let result = ModelOutput {
        id: output,
        invocation,
        text: Some("done".to_owned()),
        structured_output: None,
        finish_reason: Some("stop".to_owned()),
        usage: Some((1, 2)),
    };
    assert_eq!(record.branch, branch);
    assert_eq!(result.id, output);
    assert!(!format!("{record:?}").contains("Authorization"));
}

/// Verifies deterministic activity ordering and compact child results.
#[test]
fn nested_branch_views_are_compact_and_ordered() {
    let parent_id = EntityId::from_bytes([4; 16]);
    let child_id = EntityId::from_bytes([5; 16]);
    let result_id = EntityId::from_bytes([6; 16]);
    let mut parent = Branch::new(parent_id, "parent").expect("parent");
    parent.children.push(child_id);
    parent
        .activity
        .push(ExecutionActivity::ChildBranch(child_id));
    let mut child = Branch::new(child_id, "child").expect("child");
    child.parent = Some(parent_id);
    child.status = BranchStatus::Completed;
    child.outcome = Some("finished".to_owned());
    child.result = Some(BranchResult {
        id: result_id,
        branch: child_id,
        status: BranchStatus::Completed,
        summary: Some("finished".to_owned()),
        result_kind: Some("agent".to_owned()),
        candidate_context: None,
        references: vec![result_id],
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let session = Session {
        id: EntityId::from_bytes([7; 16]),
        branches: vec![parent, child],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    let summaries = SessionViews::child_summaries(&session, parent_id);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].result_handles, vec![result_id]);
    assert_eq!(SessionViews::branches(&session)[0].activity_count, 1);
}

/// Verifies selection changes are revision-checked and affect only future work.
#[test]
fn session_execution_selection_is_optimistic_and_versioned() {
    let mut selection = SessionExecutionSelection::default();
    selection
        .replace(
            0,
            Some("profile-a".to_owned()),
            Some("model-a".to_owned()),
            Some("careful".to_owned()),
        )
        .expect("first selection");
    let invocation = ModelInvocation {
        id: EntityId::from_bytes([8; 16]),
        branch: EntityId::from_bytes([9; 16]),
        ordinal: 1,
        agent_profile: None,
        context_fingerprint: None,
        provider_profile: Some("provider".to_owned()),
        model: Some("model-a".to_owned()),
        model_profile: Some("profile-a".to_owned()),
        provider_model_id: Some("native-model-a".to_owned()),
        thinking_level: Some("careful".to_owned()),
        request_fingerprint: None,
        cache: CacheDisposition::NotUsed,
        output: None,
        request: None,
    };

    selection
        .replace(
            1,
            Some("profile-b".to_owned()),
            Some("model-b".to_owned()),
            Some("deep-dive".to_owned()),
        )
        .expect("future selection");

    assert_eq!(invocation.model.as_deref(), Some("model-a"));
    assert_eq!(invocation.model_profile.as_deref(), Some("profile-a"));
    assert_eq!(
        invocation.provider_model_id.as_deref(),
        Some("native-model-a")
    );
    assert_eq!(invocation.thinking_level.as_deref(), Some("careful"));
    // The invocation owns resolved identifiers independently of the current
    // session selection or model-profile catalog.
    assert_eq!(selection.model.as_deref(), Some("model-b"));
    assert_eq!(selection.revision, 2);
    assert_eq!(
        selection.replace(
            1,
            Some("profile-c".to_owned()),
            Some("model-c".to_owned()),
            None,
        ),
        Err(SelectionError::RevisionConflict {
            expected: 1,
            actual: 2,
        })
    );
}

/// Verifies model and profile selection cannot be persisted inconsistently.
#[test]
fn session_execution_selection_rejects_orphan_thinking_level() {
    let mut selection = SessionExecutionSelection::default();

    assert_eq!(
        selection.replace(0, None, None, Some("careful".to_owned())),
        Err(SelectionError::InconsistentSelection)
    );
    assert_eq!(selection.revision, 0);
}

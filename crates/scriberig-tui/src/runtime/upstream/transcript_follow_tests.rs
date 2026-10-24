//! Pins the transcript viewport to the newest active-conversation rows.

use std::sync::{Arc, Mutex};

use super::synchronize_transcript_anchor;

#[path = "transcript_follow_support.rs"]
mod support;
use support::{
    COMPOSER_ROW, NEWEST_ROW, TERMINAL_ROWS, following_state, paint, publish_newest_completion,
    session_with_newest_continuation, shell,
};

/// A conversation taller than the terminal must keep its newest row, and the composer,
/// inside the painted frame instead of pinning the transcript to the oldest rows.
#[test]
fn the_active_conversation_follows_its_newest_row() {
    let session = session_with_newest_continuation(20, true);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let projected = following_state(&session, continuation);
    let rows = projected
        .conversation
        .as_ref()
        .expect("conversation projection")
        .rows
        .len();
    assert!(
        rows > TERMINAL_ROWS,
        "the fixture must exceed the painted terminal height: {rows}"
    );
    let (root, transcript) = shell(Arc::new(Mutex::new(projected)));

    let lines = paint(&root);

    assert!(
        transcript.is_following_end(),
        "the active conversation must follow the end of the transcript"
    );
    assert!(
        lines.iter().any(|line| line.contains(NEWEST_ROW)),
        "newest row must be painted: {}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|line| line.contains(COMPOSER_ROW)),
        "the composer must stay inside the terminal: {}",
        lines.join("\n")
    );
    assert_eq!(
        lines.len(),
        TERMINAL_ROWS,
        "the shell must fit the terminal"
    );
}

/// A reader who scrolls back through history keeps that position when the operation
/// completes, so the selected row and its reply anchor stay on screen.
#[test]
fn a_manual_scroll_back_through_history_is_not_yanked_to_the_newest_row() {
    let session = session_with_newest_continuation(20, false);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let shared = Arc::new(Mutex::new(following_state(&session, continuation)));
    let (root, transcript) = shell(Arc::clone(&shared));
    paint(&root);

    transcript.scroll_by(-20);
    let history_row = paint(&root)[3].trim().to_owned();
    assert!(
        !transcript.is_following_end(),
        "a manual scroll must release automatic following"
    );

    {
        let mut state = shared.lock().expect("presentation state");
        let snapshot = state
            .session_snapshot
            .as_mut()
            .expect("durable session snapshot");
        publish_newest_completion(snapshot);
        crate::state::refresh_conversation_projection(&mut state);
    }
    let browsed = paint(&root);

    assert!(
        browsed
            .iter()
            .any(|line| line.trim() == history_row.as_str()),
        "the browsed history must stay visible: {}",
        browsed.join("\n")
    );
    assert!(
        !browsed.iter().any(|line| line.contains(NEWEST_ROW)),
        "a released transcript must not jump to the newest row: {}",
        browsed.join("\n")
    );
}

/// Switching views re-anchors the shared transcript: a long auxiliary view keeps
/// reading from its first row, and returning to the conversation resumes following the
/// newest row.
#[test]
fn switching_views_anchors_the_shared_transcript_pane() {
    let session = session_with_newest_continuation(20, true);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let shared = Arc::new(Mutex::new(following_state(&session, continuation)));
    let (root, transcript) = shell(Arc::clone(&shared));
    let following = paint(&root);
    assert!(
        following.iter().any(|line| line.contains(NEWEST_ROW)),
        "the conversation starts by following its newest row: {}",
        following.join("\n")
    );
    let mut anchored = Some(crate::selection::PrimaryView::Conversation);

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Changes;
        state.review =
            crate::inspector::LoadState::Ready(crate::view::changes::ChangeReviewModel {
                base_snapshot: scriberig_core::SnapshotId::new([4; 32]),
                candidate_snapshot: None,
                change: "candidate-review-anchor".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                deltas: Vec::new(),
                details: (0..40)
                    .map(|index| format!("detail {index}: update crates/example/src/lib.rs"))
                    .collect(),
                conflicts: Vec::new(),
                validation: crate::view::changes::ReviewValidation::Passed,
                validation_profile: None,
                requires_confirmation: true,
            });
    }
    synchronize_transcript_anchor(
        &transcript,
        crate::selection::PrimaryView::Changes,
        &mut anchored,
    );
    let auxiliary = paint(&root);

    assert!(
        !transcript.is_following_end(),
        "an auxiliary view must stay anchored to its first row"
    );
    assert!(
        auxiliary
            .iter()
            .any(|line| line.contains("Candidate review")),
        "the auxiliary view must show its first row: {}",
        auxiliary.join("\n")
    );
    assert!(
        !auxiliary.iter().any(|line| line.contains(NEWEST_ROW)),
        "the auxiliary view must not follow the newest conversation row: {}",
        auxiliary.join("\n")
    );

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Conversation;
        state.review = crate::inspector::LoadState::NotRequested;
    }
    synchronize_transcript_anchor(
        &transcript,
        crate::selection::PrimaryView::Conversation,
        &mut anchored,
    );
    let resumed = paint(&root);

    assert!(
        resumed.iter().any(|line| line.contains(NEWEST_ROW)),
        "returning to the conversation resumes following the newest row: {}",
        resumed.join("\n")
    );
}

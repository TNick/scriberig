//! Pins the transcript viewport to the newest active-conversation rows.

use std::sync::{Arc, Mutex};

#[path = "transcript_follow_support.rs"]
mod support;
use support::{
    CANDIDATE_DIFF_TAIL, CANDIDATE_REVIEW, CHANGE_HANDLE, COMPOSER_ROW, NEWEST_ROW, TERMINAL_ROWS,
    candidate_review_model, following_state, paint, publish_newest_completion,
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
    let shared = Arc::new(Mutex::new(projected));
    let panes = shell(&shared);

    let lines = paint(&panes);

    assert!(
        panes.transcript().is_following_end(),
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
    let panes = shell(&shared);
    paint(&panes);

    panes.transcript().scroll_by(-20);
    let history_row = paint(&panes)[3].trim().to_owned();
    assert!(
        !panes.transcript().is_following_end(),
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
    let browsed = paint(&panes);

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

/// A view change re-anchors the shared shell: the candidate review document opens in the
/// document pane reading from its first row, and returning to the conversation resumes
/// following the newest conversation row.
#[test]
fn switching_views_anchors_the_viewport_pane() {
    let session = session_with_newest_continuation(20, true);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let shared = Arc::new(Mutex::new(following_state(&session, continuation)));
    let mut panes = shell(&shared);
    let following = paint(&panes);
    assert!(
        following.iter().any(|line| line.contains(NEWEST_ROW)),
        "the conversation starts by following its newest row: {}",
        following.join("\n")
    );

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Changes;
        state.review = crate::inspector::LoadState::Ready(candidate_review_model(20));
    }
    panes.synchronize(crate::selection::PrimaryView::Changes);
    let auxiliary = paint(&panes);

    assert!(
        !panes.document().is_following_end(),
        "the review document pane must not follow the end"
    );
    assert!(
        auxiliary.iter().any(|line| line.contains(CANDIDATE_REVIEW)),
        "the review document must show its first row: {}",
        auxiliary.join("\n")
    );
    assert!(
        !auxiliary.iter().any(|line| line.contains(NEWEST_ROW)),
        "the review document must not follow the newest conversation row: {}",
        auxiliary.join("\n")
    );

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Conversation;
        state.review = crate::inspector::LoadState::NotRequested;
    }
    panes.synchronize(crate::selection::PrimaryView::Conversation);
    let resumed = paint(&panes);

    assert!(
        resumed.iter().any(|line| line.contains(NEWEST_ROW)),
        "returning to the conversation resumes following the newest row: {}",
        resumed.join("\n")
    );
}

/// The candidate review opens while its review request is still loading, so the durable
/// candidate diff arrives without another view change. Its status evidence must stay
/// inside the painted frame at the native PTY geometry, and the whole diff must stay
/// reachable by scrolling the same pane.
#[test]
fn the_candidate_review_keeps_its_status_evidence_when_its_diff_arrives() {
    let session = session_with_newest_continuation(20, true);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let shared = Arc::new(Mutex::new(following_state(&session, continuation)));
    let mut panes = shell(&shared);
    paint(&panes);

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Changes;
        state.review = crate::inspector::LoadState::Loading { request_id: 7 };
    }
    panes.synchronize(crate::selection::PrimaryView::Changes);
    let loading = paint(&panes);
    assert!(
        loading.iter().any(|line| line.contains("Loading Review")),
        "the review request state must be painted: {}",
        loading.join("\n")
    );

    {
        let mut state = shared.lock().expect("presentation state");
        state.review = crate::inspector::LoadState::Ready(candidate_review_model(24));
    }
    let reviewed = paint(&panes);

    for evidence in [
        CANDIDATE_REVIEW,
        CHANGE_HANDLE,
        "Base snapshot:",
        "Result snapshot:",
        "Validation: Passed",
        "profile: project-ingestion-e2e",
        "old:",
    ] {
        assert!(
            reviewed.iter().any(|line| line.contains(evidence)),
            "the candidate review must keep {evidence} visible: {}",
            reviewed.join("\n")
        );
    }
    assert!(
        !reviewed
            .iter()
            .any(|line| line.contains(CANDIDATE_DIFF_TAIL)),
        "the fixture diff must stay taller than the painted viewport: {}",
        reviewed.join("\n")
    );

    panes
        .document()
        .scroll_by(i32::try_from(TERMINAL_ROWS).expect("page fits i32"));
    let next_page = paint(&panes);
    assert!(
        panes.document().scroll_top() > 0,
        "the review pane must scroll inside the document"
    );
    assert!(
        next_page
            .iter()
            .any(|line| line.contains("PHP domain source for effect 5")),
        "a page of manual scrolling must reveal the deeper candidate effects: {}",
        next_page
            .iter()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    );
    panes.document().scroll_by(i32::MAX);
    let browsed = paint(&panes);
    assert!(
        browsed
            .iter()
            .any(|line| line.contains(CANDIDATE_DIFF_TAIL)),
        "manual scrolling must reach the end of the diff without truncating it: {}",
        browsed
            .iter()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The same review document must keep its status evidence inside a narrower terminal,
/// because a reader can resize the native product window.
#[test]
fn the_candidate_review_status_evidence_survives_a_narrower_terminal() {
    let session = session_with_newest_continuation(4, true);
    let continuation = session
        .branches
        .last()
        .map(|branch| branch.id)
        .expect("continuation branch");
    let shared = Arc::new(Mutex::new(following_state(&session, continuation)));
    let mut panes = shell(&shared);

    {
        let mut state = shared.lock().expect("presentation state");
        state.primary_view = crate::selection::PrimaryView::Changes;
        state.review = crate::inspector::LoadState::Loading { request_id: 7 };
    }
    panes.synchronize(crate::selection::PrimaryView::Changes);
    paint(&panes);

    {
        let mut state = shared.lock().expect("presentation state");
        state.review = crate::inspector::LoadState::Ready(candidate_review_model(24));
    }
    let reviewed = support::paint_at(&panes, 80, 38);

    for evidence in [
        CANDIDATE_REVIEW,
        CHANGE_HANDLE,
        "Validation: Passed",
        "profile: project-ingestion-e2e",
        "old:",
    ] {
        assert!(
            reviewed.iter().any(|line| line.contains(evidence)),
            "an 80-column review must keep {evidence} visible: {}",
            reviewed.join("\n")
        );
    }
}

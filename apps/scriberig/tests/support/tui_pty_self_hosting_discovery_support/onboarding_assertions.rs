//! Verifies durable self-hosting cancellation results and unchanged project state.

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;

use super::OnboardingEvidence;

/// Verifies cancellation leaves canonical project state and copied source bytes unchanged.
///
/// - `evidence`: Onboarded project identities and exact source bytes.
pub(crate) fn assert_project_and_checkout_unchanged(evidence: &OnboardingEvidence) {
    assert_eq!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&evidence.database).expect("reopen project store"),
        ))
        .workspace(evidence.project)
        .expect("project remains active")
        .snapshot,
        evidence.base
    );
    assert_eq!(
        fs::read(evidence.checkout.join("Cargo.toml")).expect("manifest"),
        evidence.manifest
    );
    assert_eq!(
        fs::read(
            evidence
                .checkout
                .join("crates/scriberig-project/src/scan.rs")
        )
        .expect("scanner"),
        evidence.scanner
    );
    assert!(
        !evidence
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
}

/// Waits for the session store to persist an agent cancellation result.
///
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
///
/// Session with a durable cancelled branch result.
/// # Returns
/// The operation result.
pub(crate) fn wait_for_cancellation(
    database: &std::path::Path,
    session: EntityId,
) -> scriberig_session::Session {
    let store = SqliteSessionStore::open(database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let saved = store
            .load(session)
            .expect("load session")
            .expect("durable session");
        if saved.branches.iter().any(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
        }) {
            return saved;
        }
        let branches = saved
            .branches
            .iter()
            .map(|branch| {
                (
                    branch.title.as_str(),
                    branch.status,
                    branch
                        .result
                        .as_ref()
                        .and_then(|result| result.result_kind.as_deref()),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "cancelled result was not persisted: {branches:?}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

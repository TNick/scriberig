//! Publishes provider-visible reasoning-summary activity for one TUI root branch.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use scriberig_app::OperationRegistry;
use scriberig_core::OperationId;
use scriberig_session::{AgentActivityKind, AgentActivityState};

/// Maximum summary characters retained for transient presentation.
const MAX_SUMMARY_CHARS: usize = 256;

/// Host sink retaining one invocation's bounded reasoning-summary progress.
///
/// The runtime only forwards upstream `ReasoningSummaryDelta` text here; raw
/// thinking deltas and hidden chain-of-thought never reach this sink.
///
/// # Public members
/// None.
///
/// # Private members
/// - `registry`: Shared operation event registry.
/// - `operation`: Active operation identity.
/// - `branch`: Durable branch identity associated with the activity.
/// - `text`: Accumulated safe summary text.
/// - `ordinal`: Last published per-invocation arrival ordinal.
pub(super) struct ReasoningSummaryActivity {
    /// Registry receiving bounded transient activity events.
    registry: OperationRegistry,
    /// Operation that owns the activity event.
    operation: OperationId,
    /// Durable branch identity for the activity.
    branch: String,
    /// Accumulated provider-visible summary text.
    text: Mutex<String>,
    /// Highest observed per-invocation ordinal.
    ordinal: AtomicU64,
}

impl ReasoningSummaryActivity {
    /// Creates a summary sink for one accepted root invocation.
    ///
    /// # Parameters
    /// - `registry`: Operation registry receiving activity events.
    /// - `operation`: Exact active operation identity.
    /// - `branch`: Exact durable root branch identity.
    ///
    /// # Returns
    /// A shared sink suitable for the runtime summary callback.
    pub(super) fn new(
        registry: OperationRegistry,
        operation: OperationId,
        branch: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            registry,
            operation,
            branch,
            text: Mutex::new(String::new()),
            ordinal: AtomicU64::new(0),
        })
    }

    /// Returns a runtime callback that records safe summary fragments in order.
    ///
    /// # Returns
    /// A callback accepting an invocation ordinal and safe summary delta.
    pub(super) fn callback(self: &Arc<Self>) -> scriberig_agent_runtime::ReasoningSummaryCallback {
        let sink = Arc::clone(self);
        Arc::new(move |ordinal, delta| sink.observe(ordinal, delta))
    }

    /// Publishes the settled summary once the invocation returns.
    pub(super) fn settle(&self) {
        let Ok(text) = self.text.lock() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let summary = text.clone();
        drop(text);
        let ordinal = self
            .ordinal
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let _ = self.registry.publish_agent_activity(
            &self.operation,
            self.branch.clone(),
            AgentActivityKind::ReasoningSummary,
            AgentActivityState::Completed,
            summary,
            ordinal,
        );
    }

    /// Appends one safe fragment and publishes bounded progress.
    ///
    /// # Parameters
    /// - `ordinal`: Monotonic per-invocation arrival ordinal.
    /// - `delta`: Safe provider-visible summary fragment.
    fn observe(&self, ordinal: u64, delta: &str) {
        let Ok(mut text) = self.text.lock() else {
            return;
        };
        text.push_str(delta);
        if text.chars().count() > MAX_SUMMARY_CHARS {
            *text = text.chars().take(MAX_SUMMARY_CHARS).collect();
        }
        let summary = text.clone();
        drop(text);
        self.ordinal.fetch_max(ordinal, Ordering::Relaxed);
        let _ = self.registry.publish_agent_activity(
            &self.operation,
            self.branch.clone(),
            AgentActivityKind::ReasoningSummary,
            AgentActivityState::Running,
            summary,
            ordinal,
        );
    }
}

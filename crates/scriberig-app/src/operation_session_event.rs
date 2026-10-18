//! Publishes non-operation session lifecycle notifications.

use crate::{ApplicationEvent, OperationRegistry};
use scriberig_core::EntityId;

impl OperationRegistry {
    /// Publishes a bounded selection change without creating an agent operation.
    ///
    /// # Parameters
    /// - `self`: Registry that owns event subscribers.
    /// - `session`: Stable identity of the updated session.
    /// - `activity_kind`: Safe model or thinking selection category.
    /// - `summary`: Safe user-facing selection summary.
    pub fn publish_session_selection_changed(
        &self,
        session: EntityId,
        activity_kind: scriberig_session::AgentActivityKind,
        summary: &str,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.publish(&ApplicationEvent::SessionSelectionChanged {
                session,
                activity_kind,
                summary: summary.chars().take(256).collect(),
            });
        }
    }

    /// Publishes a session change so subscribed clients can refresh state.
    ///
    /// # Parameters
    /// - `self`: Registry that owns event subscribers.
    /// - `session`: Stable identity of the updated session.
    pub fn publish_session_changed(&self, session: EntityId) {
        if let Ok(mut state) = self.state.lock() {
            state.publish(&ApplicationEvent::SessionChanged { session });
        }
    }
}

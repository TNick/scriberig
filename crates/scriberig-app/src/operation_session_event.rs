//! Publishes non-operation session lifecycle notifications.

use crate::{ApplicationEvent, OperationRegistry};
use scriberig_core::EntityId;

impl OperationRegistry {
    /// Publishes a bounded selection change without creating an agent operation.
    ///
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
    /// - `self`: Registry that owns event subscribers.
    /// - `session`: Stable identity of the updated session.
    pub fn publish_session_changed(&self, session: EntityId) {
        if let Ok(mut state) = self.state.lock() {
            state.publish(&ApplicationEvent::SessionChanged { session });
        }
    }

    /// Publishes one typed settled plan transition after durable mutation.
    ///
    /// - `self`: Registry that owns event subscribers.
    /// - `session`: Stable owning session identity.
    /// - `plan`: Stable plan identity.
    /// - `node`: Stable plan node identity when applicable.
    /// - `activity_kind`: Typed plan transition category.
    /// - `state`: Settled lifecycle state represented by durable session data.
    /// - `summary`: Safe concise text for client activity projections.
    pub fn publish_plan_activity(
        &self,
        session: EntityId,
        plan: EntityId,
        node: Option<EntityId>,
        activity_kind: scriberig_session::SessionActivityKind,
        state: scriberig_session::SessionActivityState,
        summary: &str,
    ) {
        if let Ok(mut event_state) = self.state.lock() {
            event_state.publish(&ApplicationEvent::PlanActivity {
                session,
                plan,
                node,
                activity_kind,
                state,
                summary: summary.chars().take(256).collect(),
            });
        }
    }
}

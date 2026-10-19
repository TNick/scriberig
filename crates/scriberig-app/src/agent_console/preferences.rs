//! Defines user-scoped, shared agent-console display preferences.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::ApplicationError;

/// Controls whether safe reasoning summaries and runtime status are visible.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningVisibility {
    /// Omit reasoning summaries and status rows.
    #[default]
    Hidden,
    /// Show safe user-facing reasoning summaries and status rows.
    Shown,
}

/// Controls the detail used to present captured tool invocations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInvocationDetail {
    /// Show a compact command-like line.
    #[default]
    Simple,
    /// Show bounded schema-redacted structured arguments.
    Detailed,
}

/// Controls the amount of captured tool result data shown by default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputVisibility {
    /// Hide result content while preserving invocation and status.
    Hidden,
    /// Show only deterministic safe result summaries.
    #[default]
    Summary,
    /// Show bounded safe previews and allow explicit expansion.
    Full,
}

/// Controls whether captured safe model-request sections are visible.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptVisibility {
    /// Omit captured model-request sections.
    #[default]
    Hidden,
    /// Show bounded captured model-request sections.
    Shown,
}

/// Shared user defaults for presenting already captured agent activity.
///
/// # Public members
/// - `reasoning`: Safe reasoning-summary and runtime-status visibility.
/// - `tool_invocation`: Tool invocation detail mode.
/// - `tool_output`: Tool result visibility mode.
/// - `prompt`: Captured model-request visibility.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AgentPresentationPreferences {
    /// Safe reasoning-summary and runtime-status visibility.
    pub reasoning: ReasoningVisibility,
    /// Tool invocation detail mode.
    pub tool_invocation: ToolInvocationDetail,
    /// Tool result visibility mode.
    pub tool_output: ToolOutputVisibility,
    /// Captured model-request visibility.
    pub prompt: PromptVisibility,
}

/// One revisioned snapshot of shared presentation defaults.
///
/// # Public members
/// - `revision`: Optimistic mutation revision.
/// - `preferences`: Shared user defaults.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AgentPresentationPreferenceSnapshot {
    /// Optimistic mutation revision.
    pub revision: u64,
    /// Shared user defaults.
    pub preferences: AgentPresentationPreferences,
}

/// Stores shared agent-console preferences independently from session history.
pub trait AgentPresentationPreferenceStore: Send + Sync {
    /// Reads one user's shared presentation defaults.
    ///
    /// - `principal`: Stable user identity.
    ///
    /// The current preference snapshot, using defaults when absent.
    ///
    /// Returns an application storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(
        &self,
        principal: &str,
    ) -> Result<AgentPresentationPreferenceSnapshot, ApplicationError>;

    /// Replaces shared defaults only when the caller's revision is current.
    ///
    /// - `principal`: Stable user identity.
    /// - `expected_revision`: Snapshot revision observed by the caller.
    /// - `preferences`: Complete independent display choices.
    ///
    /// The updated authoritative snapshot.
    ///
    /// Returns a conflict for stale revisions or a storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap(
        &self,
        principal: &str,
        expected_revision: u64,
        preferences: AgentPresentationPreferences,
    ) -> Result<AgentPresentationPreferenceSnapshot, ApplicationError>;
}

/// Provides process-shared user preferences when no durable store is wired.
#[derive(Default)]
pub struct InMemoryAgentPresentationPreferenceStore {
    snapshots: Mutex<BTreeMap<String, AgentPresentationPreferenceSnapshot>>,
}

impl AgentPresentationPreferenceStore for InMemoryAgentPresentationPreferenceStore {
    /// Reads one user's shared presentation defaults.
    ///
    /// - `self`: Shared in-memory store.
    /// - `principal`: Stable user identity.
    ///
    /// The stored snapshot or built-in defaults.
    ///
    /// Returns an error when the store lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read(
        &self,
        principal: &str,
    ) -> Result<AgentPresentationPreferenceSnapshot, ApplicationError> {
        let snapshots = self
            .snapshots
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "presentation preference lock failed".to_owned(),
            })?;
        Ok(snapshots.get(principal).cloned().unwrap_or_default())
    }

    /// Replaces shared defaults when the observed revision is current.
    ///
    /// - `self`: Shared in-memory store.
    /// - `principal`: Stable user identity.
    /// - `expected_revision`: Snapshot revision observed by the caller.
    /// - `preferences`: Complete independent display choices.
    ///
    /// The updated authoritative snapshot.
    ///
    /// Returns a conflict for stale revisions or a poisoned-lock error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap(
        &self,
        principal: &str,
        expected_revision: u64,
        preferences: AgentPresentationPreferences,
    ) -> Result<AgentPresentationPreferenceSnapshot, ApplicationError> {
        let mut snapshots = self
            .snapshots
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "presentation preference lock failed".to_owned(),
            })?;
        let current = snapshots.get(principal).cloned().unwrap_or_default();
        if current.revision != expected_revision {
            return Err(ApplicationError::Conflict {
                subject: format!("agent presentation preferences for {principal}"),
                details: Box::new(crate::ConflictDetails {
                    operation: Some(format!(
                        "stale preference revision {expected_revision}; current revision is {}",
                        current.revision
                    )),
                    ..crate::ConflictDetails::default()
                }),
            });
        }
        let updated = AgentPresentationPreferenceSnapshot {
            revision: current.revision.checked_add(1).ok_or_else(|| {
                ApplicationError::Internal {
                    message: "presentation preference revision is exhausted".to_owned(),
                }
            })?,
            preferences,
        };
        snapshots.insert(principal.to_owned(), updated.clone());
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentPresentationPreferenceStore, AgentPresentationPreferences,
        InMemoryAgentPresentationPreferenceStore, PromptVisibility, ToolOutputVisibility,
    };

    /// Verifies preferences are user-scoped and mutations reject stale revisions.
    #[test]
    fn preferences_are_scoped_and_revision_checked() {
        let store = InMemoryAgentPresentationPreferenceStore::default();
        let defaults = store.read("user-a").expect("default preferences");
        assert_eq!(defaults.revision, 0);
        assert_eq!(
            defaults.preferences,
            AgentPresentationPreferences::default()
        );

        let updated = store
            .compare_and_swap(
                "user-a",
                0,
                AgentPresentationPreferences {
                    tool_output: ToolOutputVisibility::Full,
                    prompt: PromptVisibility::Shown,
                    ..AgentPresentationPreferences::default()
                },
            )
            .expect("update preferences");
        assert_eq!(updated.revision, 1);
        assert_eq!(
            store
                .read("user-b")
                .expect("other user defaults")
                .preferences,
            AgentPresentationPreferences::default()
        );
        assert_eq!(
            store
                .compare_and_swap("user-a", 0, AgentPresentationPreferences::default())
                .expect_err("stale update must conflict")
                .category(),
            "conflict"
        );
    }
}

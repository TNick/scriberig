//! Defines provider-neutral session entities and progressive views.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod branch;
pub mod description;
pub mod error;
pub mod execution;
pub mod schema;
pub mod turn;
pub mod types;
pub mod views;

/// Defines the durable session repository boundary.
pub mod repository {
    use crate::Session;
    use scriberig_core::EntityId;

    /// Persists provider-neutral session graphs without depending on a model runtime.
    pub trait SessionStore: Send + Sync {
        /// Loads one exact session graph when it exists.
        ///
        /// # Parameters
        /// - `session`: Stable session identity.
        ///
        /// # Returns
        /// The stored session, or `None` when no record exists.
        ///
        /// # Errors
        /// Returns a backend diagnostic that is safe for the application layer.
        fn load(&self, session: EntityId) -> Result<Option<Session>, String>;

        /// Stores one complete session graph atomically.
        ///
        /// # Parameters
        /// - `session`: Session graph to persist.
        ///
        /// # Errors
        /// Returns a backend diagnostic when persistence fails.
        ///
        /// # Returns
        /// Unit after the session is persisted.
        fn save(&self, session: &Session) -> Result<(), String>;
    }
}

pub use branch::{Branch, BranchStatus};
pub use description::Description;
pub use error::SessionError;
pub use execution::{CacheDisposition, ExecutionActivity, ModelInvocation, ModelOutput};
pub use repository::SessionStore;
pub use turn::{AgentOutput, HumanInput, ToolInvocation, ToolResult, Turn};
pub use types::{BranchResult, Decision, Goal, Observation, Question, Session, Summary, Task};
pub use views::{BranchDescriptor, ChildBranchSummary, DecisionEntry, SessionViews, TurnView};

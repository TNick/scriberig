//! Defines provider-neutral session entities and progressive views.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]
pub mod branch;
pub mod description;
pub mod error;
pub mod schema;
pub mod turn;
pub mod types;
pub mod views;
pub use branch::{Branch, BranchStatus};
pub use description::Description;
pub use error::SessionError;
pub use turn::{AgentOutput, HumanInput, ToolInvocation, ToolResult, Turn};
pub use types::{BranchResult, Decision, Goal, Observation, Question, Session, Summary, Task};
pub use views::{BranchDescriptor, DecisionEntry, SessionViews, TurnView};

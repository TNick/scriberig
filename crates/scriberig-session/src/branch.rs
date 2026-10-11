//! Defines branch lifecycle, compact metadata, and turn ownership.
use crate::{description::Description, error::SessionError, turn::Turn, types::BranchResult};
use scriberig_core::EntityId;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
/// Session-domain structured value.
pub enum BranchStatus {
    Active,
    Completed,
    Abandoned,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Branch {
    pub id: EntityId,
    pub title: String,
    pub purpose: Option<String>,
    pub outcome: Option<String>,
    pub status: BranchStatus,
    pub description: Option<Description>,
    pub touched: Vec<EntityId>,
    pub turns: Vec<Turn>,
    pub result: Option<BranchResult>,
}
impl Branch {
    /// Creates an active branch.
    /// # Errors
    /// Returns `EmptyTitle` for a blank title.
    /// Executes `new` for structured session state.
    ///
    /// # Parameters
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    pub fn new(id: EntityId, title: impl Into<String>) -> Result<Self, SessionError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(SessionError::EmptyTitle);
        }
        Ok(Self {
            id,
            title,
            purpose: None,
            outcome: None,
            status: BranchStatus::Active,
            description: None,
            touched: Vec::new(),
            turns: Vec::new(),
            result: None,
        })
    }
    /// Applies a lifecycle transition.
    /// # Errors
    /// Returns `InvalidTransition` when a terminal branch changes.
    /// Executes `transition` for structured session state.
    ///
    /// # Parameters
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    pub fn transition(&mut self, next: BranchStatus) -> Result<(), SessionError> {
        let valid = matches!(
            (self.status, next),
            (
                BranchStatus::Active,
                BranchStatus::Completed | BranchStatus::Abandoned
            )
        );
        if !valid && self.status != next {
            return Err(SessionError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        Ok(())
    }
    /// Appends an explicitly ordered turn.
    /// Executes `push_turn` for structured session state.
    ///
    /// # Parameters
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    pub fn push_turn(&mut self, mut turn: Turn) {
        turn.ordinal = self.turns.len() as u64;
        self.turns.push(turn);
    }
    /// Adds a cross-domain semantic handle.
    /// Executes `touch` for structured session state.
    ///
    /// # Parameters
    /// - `self`: Session value or formatter.
    /// - `id`: Stable entity identifier when applicable.
    /// - `title`: Branch title when applicable.
    /// - `next`: Requested lifecycle state when applicable.
    /// - `turn`: Turn to append when applicable.
    /// - `entity`: Cross-domain entity to reference when applicable.
    /// - `session`: Session snapshot when applicable.
    /// - `branch`: Branch snapshot when applicable.
    /// - `decisions`: Decision entries when applicable.
    /// - `f`: Formatting destination when applicable.
    ///
    /// # Returns
    /// The structured result.
    ///
    /// # Errors
    /// Returns a validation or formatting error when inputs are invalid.
    pub fn touch(&mut self, entity: EntityId) {
        if !self.touched.contains(&entity) {
            self.touched.push(entity);
            self.touched.sort();
        }
    }
}

//! Defines watcher-neutral filesystem observation hints.
use crate::ProjectionLocation;
/// A watcher-neutral filesystem observation hint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Observation {
    Created(ProjectionLocation),
    Modified(ProjectionLocation),
    Removed(ProjectionLocation),
    Renamed {
        from: ProjectionLocation,
        to: ProjectionLocation,
    },
    RescanRequired(ProjectionLocation),
}

/// Supplies watcher-neutral observations to a synchronization coordinator.
pub trait ObservationSource {
    /// Error returned while obtaining the next observation.
    type Error;

    /// Returns the next observation, or `None` when the source is exhausted.
    ///
    /// # Errors
    /// Returns the source-specific delivery error.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn next(&mut self) -> Result<Option<Observation>, Self::Error>;
}

/// Deterministic observation source for tests and embedded callers.
#[derive(Clone, Debug, Default)]
pub struct VecObservationSource {
    observations: std::collections::VecDeque<Observation>,
}

impl VecObservationSource {
    /// Creates a source from a finite sequence of observations.
    ///
    /// # Parameters
    /// - `observations`: Observations to deliver in order.
    ///
    /// # Returns
    /// A deterministic observation source.
    #[must_use]
    pub fn new(observations: impl IntoIterator<Item = Observation>) -> Self {
        Self {
            observations: observations.into_iter().collect(),
        }
    }
}

impl ObservationSource for VecObservationSource {
    type Error = std::convert::Infallible;

    /// Delivers the next queued observation.
    ///
    /// # Returns
    /// The next observation, or `None` after the queue is empty.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn next(&mut self) -> Result<Option<Observation>, Self::Error> {
        Ok(self.observations.pop_front())
    }
}

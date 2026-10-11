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

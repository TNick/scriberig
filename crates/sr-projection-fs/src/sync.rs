//! Defines backend-independent synchronization state transitions.
use crate::{
    hash::FileDigest,
    hash::digest_bytes,
    io::{FileSystem, IoError},
    location::ProjectionLocation,
    observe::Observation,
    ownership::ProjectionOwnership,
};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};
/// Runtime synchronization state for one projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncState {
    pub projection: ProjectionRef,
    pub location: ProjectionLocation,
    pub ownership: ProjectionOwnership,
    pub base: SemanticStateRef,
    pub rendered: FileDigest,
    pub observed: Option<FileDigest>,
    pub status: ProjectionStatus,
}
/// In-memory synchronization state boundary.
#[derive(Clone, Debug, Default)]
pub struct MemorySyncStore {
    states: Vec<SyncState>,
}
impl MemorySyncStore {
    /// Creates or replaces a state record.
    ///
    /// # Parameters
    /// - `state`: State to store.
    pub fn put(&mut self, state: SyncState) {
        if let Some(existing) = self
            .states
            .iter_mut()
            .find(|s| s.projection == state.projection)
        {
            *existing = state;
        } else {
            self.states.push(state);
        }
    }
    /// Finds state by location.
    ///
    /// # Parameters
    /// - `location`: Location to find.
    ///
    /// # Returns
    /// Matching state when present.
    #[must_use]
    pub fn find(&self, location: &ProjectionLocation) -> Option<&SyncState> {
        self.states.iter().find(|s| &s.location == location)
    }
    /// Finds mutable state by location.
    ///
    /// # Parameters
    /// - `location`: Location to find.
    ///
    /// # Returns
    /// Mutable matching state when present.
    pub fn find_mut(&mut self, location: &ProjectionLocation) -> Option<&mut SyncState> {
        self.states.iter_mut().find(|s| &s.location == location)
    }

    /// Moves a synchronization record when an editable projection is renamed.
    ///
    /// # Parameters
    /// - `from`: Previous physical location.
    /// - `to`: New physical location.
    ///
    /// # Returns
    /// `true` when a record was moved.
    pub fn rename(&mut self, from: &ProjectionLocation, to: ProjectionLocation) -> bool {
        let Some(state) = self.states.iter_mut().find(|state| &state.location == from) else {
            return false;
        };
        state.location = to;
        true
    }
}
/// Inspects one observation using exact current bytes and digest state.
/// # Errors
/// Returns a filesystem read error.
pub fn inspect_observation<F: FileSystem>(
    fs: &F,
    store: &mut MemorySyncStore,
    event: &Observation,
) -> Result<Option<ProjectionStatus>, IoError> {
    let location = match event {
        Observation::Created(p)
        | Observation::Modified(p)
        | Observation::Removed(p)
        | Observation::RescanRequired(p) => p,
        Observation::Renamed { from, to } => {
            if !store.rename(from, to.clone()) {
                return Ok(None);
            }
            to
        }
    };
    let Some(state) = store.find_mut(location) else {
        return Ok(None);
    };
    if matches!(event, Observation::Removed(_)) {
        state.status = ProjectionStatus::Missing;
        return Ok(Some(state.status));
    }
    let bytes = fs.read(location.path())?;
    let digest = digest_bytes(&bytes);
    state.observed = Some(digest);
    if digest == state.rendered {
        state.status = ProjectionStatus::Synchronized;
    } else {
        state.status = ProjectionStatus::ExternallyModified;
    }
    Ok(Some(state.status))
}

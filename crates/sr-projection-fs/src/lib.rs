//! Defines filesystem projection ownership, hashing, and atomic IO.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod coordinator;
pub mod hash;
mod import;
pub mod io;
pub mod location;
pub mod observe;
pub mod ownership;
pub mod recovery;
pub mod state;
pub mod sync;

pub use coordinator::{SyncCoordinator, inspect_observation};
pub use hash::{FileDigest, digest_bytes, digest_file};
pub use import::RenderError;
pub use io::{FileMetadata, FileSystem, IoError, LocalFileSystem};
pub use location::ProjectionLocation;
pub use observe::{Observation, ObservationSource, VecObservationSource};
pub use ownership::ProjectionOwnership;
pub use recovery::{FailureStage, MemoryRecoveryStore, RecoveryRecord};
pub use sr_projection::{ProjectionRef, ProjectionStatus};
pub use state::FilesystemProjectionState;
pub use sync::{ImportCompletion, ImportRequest, MemorySyncStore, ObservationResult, SyncState};

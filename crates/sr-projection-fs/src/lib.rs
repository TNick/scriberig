//! Defines filesystem projection ownership, hashing, and atomic IO.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod artifact_materialization;
pub mod coordinator;
pub use artifact_materialization::{
    ArtifactMaterializationMember, ArtifactMaterializationPlan, ArtifactMaterializationReceipt,
    ArtifactMaterializationState, ArtifactMemberReceipt, materialize_artifact,
    materialize_artifact_validated, materialize_artifact_validated_with_progress,
};
pub mod hash;
mod import;
pub mod io;
pub mod location;
pub mod observe;
pub mod opaque_content;
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
pub use opaque_content::{
    OpaqueContentError, OpaqueContentOutcome, OpaquePreservedRecord, discard_opaque_record,
    preserve_embedded_opaque_record, preserve_opaque_record, validate_opaque_preservation,
};
pub use ownership::ProjectionOwnership;
pub use recovery::{FailureStage, MemoryRecoveryStore, RecoveryRecord};
pub use sr_projection::{ProjectionRef, ProjectionStatus};
pub use state::FilesystemProjectionState;
pub use sync::{ImportCompletion, ImportRequest, MemorySyncStore, ObservationResult, SyncState};

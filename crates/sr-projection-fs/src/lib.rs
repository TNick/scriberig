//! Defines filesystem projection ownership, hashing, and atomic IO.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod hash;
pub mod io;
pub mod location;
pub mod ownership;
pub mod state;

pub use hash::{FileDigest, digest_bytes, digest_file};
pub use io::{FileMetadata, FileSystem, IoError, LocalFileSystem};
pub use location::ProjectionLocation;
pub use ownership::ProjectionOwnership;
pub use sr_projection::{ProjectionRef, ProjectionStatus};
pub use state::FilesystemProjectionState;

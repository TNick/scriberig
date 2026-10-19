//! Defines backend-neutral storage observability models.

use scriberig_core::{RootName, SnapshotId};

use crate::ProtectedRoots;

/// Machine-readable category for a storage failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreErrorCategory {
    /// Requested immutable data was not found.
    NotFound,
    /// An identifier or canonical encoding is invalid.
    InvalidEncoding,
    /// The backend or encoding version is unsupported.
    UnsupportedVersion,
    /// An optimistic root update used a stale value.
    StaleRoot,
    /// Persisted canonical data failed integrity checks.
    Integrity,
    /// The requested operation is invalid.
    InvalidOperation,
    /// The backend could not complete an I/O or transaction operation.
    Backend,
}

/// Describes a failure from a backend-neutral observability operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreOperationError {
    /// Machine-readable failure category.
    pub category: StoreErrorCategory,
    /// Human-readable diagnostic detail.
    pub message: String,
}

impl StoreOperationError {
    /// Creates a backend failure with diagnostic detail.
    ///
    /// - `message`: Human-readable backend diagnostic.
    ///
    /// A categorized backend operation error.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn backend(message: impl Into<String>) -> Self {
        Self {
            category: StoreErrorCategory::Backend,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StoreOperationError {
    /// Formats the diagnostic detail for a human reader.
    ///
    /// - `formatter`: Output formatter.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when the formatter rejects the output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.category, self.message)
    }
}

impl std::error::Error for StoreOperationError {}

impl std::fmt::Display for StoreErrorCategory {
    /// Formats a stable category name.
    ///
    /// - `formatter`: Output formatter.
    ///
    /// Formatting result.
    ///
    /// Returns a formatting error when the formatter rejects the output.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::NotFound => "not_found",
            Self::InvalidEncoding => "invalid_encoding",
            Self::UnsupportedVersion => "unsupported_version",
            Self::StaleRoot => "stale_root",
            Self::Integrity => "integrity",
            Self::InvalidOperation => "invalid_operation",
            Self::Backend => "backend",
        };
        formatter.write_str(name)
    }
}

impl crate::StoreError {
    /// Returns the backend-neutral category for this revision error.
    ///
    /// A stable machine-readable error category.
    #[must_use]
    pub const fn category(&self) -> StoreErrorCategory {
        match self {
            Self::IntegrityCollision { .. } | Self::RevisionIdentityMismatch { .. } => {
                StoreErrorCategory::Integrity
            }
            Self::InvalidCanonicalEncoding(error) => match error {
                scriberig_core::EncodingError::UnsupportedVersion { .. } => {
                    StoreErrorCategory::UnsupportedVersion
                }
            },
            Self::NotFound { .. } => StoreErrorCategory::NotFound,
            Self::InvalidOperation { .. } => StoreErrorCategory::InvalidOperation,
            Self::LockPoisoned | Self::Backend(_) => StoreErrorCategory::Backend,
        }
    }
}

impl crate::RootError {
    /// Returns the backend-neutral category for this root error.
    ///
    /// A stable machine-readable error category.
    #[must_use]
    pub const fn category(&self) -> StoreErrorCategory {
        match self {
            Self::Conflict { .. } => StoreErrorCategory::StaleRoot,
            Self::Backend(_) => StoreErrorCategory::Backend,
        }
    }
}

impl crate::SnapshotError {
    /// Returns the backend-neutral category for this snapshot error.
    ///
    /// A stable machine-readable error category.
    #[must_use]
    pub const fn category(&self) -> StoreErrorCategory {
        match self {
            Self::InvalidEncoding(_) => StoreErrorCategory::InvalidEncoding,
            Self::UnknownSnapshot(_) => StoreErrorCategory::NotFound,
            Self::Backend(_) => StoreErrorCategory::Backend,
        }
    }
}

/// Counts and optional backend storage measurements.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreStats {
    /// Number of retained immutable revisions.
    pub revisions: Option<u64>,
    /// Number of retained snapshots.
    pub snapshots: Option<u64>,
    /// Number of named roots.
    pub named_roots: Option<u64>,
    /// Backend storage bytes when cheaply available.
    pub storage_bytes: Option<u64>,
    /// Number of reachable objects from the last analysis.
    pub reachable_objects: Option<u64>,
    /// Number of unreachable candidates from the last analysis.
    pub unreachable_objects: Option<u64>,
}

/// Current status of one named root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootStatus {
    /// Root identity.
    pub name: RootName,
    /// Exact snapshot currently assigned to the root.
    pub snapshot: Option<SnapshotId>,
}

/// Result counts from garbage-collection maintenance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GcReport {
    /// Number of candidates identified by marking.
    pub candidates: u64,
    /// Number of objects deleted by sweeping.
    pub deleted: u64,
}

impl GcReport {
    /// Returns the number of objects considered by the report.
    ///
    /// Candidate object count.
    #[must_use]
    pub const fn candidates(&self) -> u64 {
        self.candidates
    }
}

/// Status of backend-derived indexes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexStatus {
    /// Whether all known derived indexes reflect authoritative data.
    pub consistent: bool,
}

/// Optional observability contract implemented by storage backends.
pub trait StoreObservability {
    /// Returns backend statistics without changing canonical state.
    ///
    /// A snapshot of available storage measurements.
    ///
    /// Returns a backend error when statistics cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn stats(&self) -> Result<StoreStats, StoreOperationError>;

    /// Returns statistics annotated with reachability from protected roots.
    ///
    /// - `protected`: Stable roots used for reachability counts.
    /// - `_protected`: Ignored by the default implementation.
    ///
    /// Available storage measurements and reachability counts.
    ///
    /// Returns a backend error when statistics cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn stats_for(&self, _protected: &ProtectedRoots) -> Result<StoreStats, StoreOperationError> {
        self.stats()
    }

    /// Lists named roots without exposing backend handles.
    ///
    /// Root status records in deterministic name order.
    ///
    /// Returns a backend error when roots cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn root_statuses(&self) -> Result<Vec<RootStatus>, StoreOperationError>;

    /// Rebuilds derived indexes without changing canonical objects.
    ///
    /// The resulting index status.
    ///
    /// Returns a backend error when rebuild fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError>;
}

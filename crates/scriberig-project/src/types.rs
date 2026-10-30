//! Defines stable project scan, import, and diagnostic result types.

use std::fmt::{Display, Formatter};

use scriberig_core::{EntityId, SnapshotId};
use scriberig_store_sqlite::ProjectRecord;

/// Selects which filesystem files a project scan persists.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanOptions {
    /// Restricts the manifest to files tracked by the Git index.
    pub git_tracked: bool,
}

/// Supplies explicit rename evidence when one scanned project path replaces another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFileRename {
    /// Previous project-relative POSIX path from the persisted scan.
    pub from_path: String,
    /// New project-relative POSIX path in the current scan.
    pub to_path: String,
}

/// Describes one scanned project file with exact source bytes and classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanFile {
    /// Project-relative POSIX path.
    pub relative_path: String,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
    /// Selected adapter identifier.
    pub adapter: String,
    /// Stable scan classification.
    pub status: String,
    /// Adapter-classified logical grouping key, when one is nominated.
    pub grouping_key: Option<String>,
}

/// Reports the persisted result of registering one project scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanReport {
    /// Registered project record.
    pub project: ProjectRecord,
    /// Persisted deterministic scan files.
    pub files: Vec<ScanFile>,
    /// Whether the manifest was restricted to Git-indexed files.
    pub git_tracked: bool,
}

/// Reports one successfully promoted project import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportReport {
    /// Project after successful import.
    pub project: ProjectRecord,
    /// Number of source files imported.
    pub files: usize,
    /// Number of semantic entities promoted.
    pub entities: usize,
    /// Promoted immutable snapshot.
    pub snapshot: SnapshotId,
}

/// Resolves the exact active semantic workspace for one imported project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectWorkspace {
    /// Canonical project catalog record.
    pub project: ProjectRecord,
    /// Current immutable project snapshot.
    pub snapshot: SnapshotId,
    /// Semantic source root used to begin navigation.
    pub root_entity: EntityId,
}

/// Describes one recoverable or terminal import diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDiagnostic {
    /// Semantic entity targeted by an operation when one is known.
    pub entity: Option<EntityId>,
    /// Project-relative file path.
    pub path: String,
    /// Adapter that produced the diagnostic.
    pub adapter: String,
    /// Import stage that failed.
    pub stage: String,
    /// Stable diagnostic category.
    pub code: String,
    /// Safe diagnostic message.
    pub message: String,
    /// Whether editing the source and retrying may succeed.
    pub retryable: bool,
}

/// Reports a project onboarding failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectError {
    /// Stable failure category.
    pub code: &'static str,
    /// Safe human-readable failure detail.
    pub message: String,
    /// Structured diagnostics suitable for agents and product surfaces.
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl Display for ProjectError {
    /// Formats the onboarding failure.
    ///
    /// # Parameters
    /// - `self`: Onboarding failure to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// Formatter status after writing the failure.
    ///
    /// # Errors
    /// Returns the formatter error when writing the failure is rejected.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProjectError {}

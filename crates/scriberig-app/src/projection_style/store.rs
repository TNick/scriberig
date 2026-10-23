//! Stores purpose-scoped projection style configuration outside canonical state.

use std::collections::BTreeMap;
use std::sync::Mutex;

use sr_projection::{ProjectionKind, StyleValue};

use crate::ApplicationError;

/// Names the configuration scope one style input belongs to.
///
/// The scope decides who may see the value: project and workspace scopes are
/// shared, user and view scopes are personal. Keeping them as distinct storage
/// scopes is what lets shared materialization exclude personal preference
/// without a flag check inside a renderer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum StyleScopeKind {
    /// Shared project configuration knowledge.
    Project,
    /// Shared workspace materialization policy.
    Workspace,
    /// Personal preference for one principal.
    User,
    /// One specific view session override.
    View,
}

impl std::fmt::Display for StyleScopeKind {
    /// Formats the scope as a stable lower-case label.
    ///
    /// - `self`: Scope to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Project => "project",
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::View => "view",
        };
        formatter.write_str(label)
    }
}

/// Identifies one stored style setting.
///
/// # Public members
/// - `scope`: Configuration scope owning the setting.
/// - `scope_id`: Stable identity inside that scope, such as a principal or project.
/// - `projection_kind`: Projection kind the setting applies to.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct StyleEntryKey {
    /// Configuration scope owning the setting.
    pub scope: StyleScopeKind,
    /// Stable identity inside that scope.
    pub scope_id: String,
    /// Projection kind the setting applies to.
    pub projection_kind: ProjectionKind,
}

impl StyleEntryKey {
    /// Builds one configuration key.
    ///
    /// - `scope`: Configuration scope owning the setting.
    /// - `scope_id`: Stable identity inside that scope.
    /// - `projection_kind`: Projection kind the setting applies to.
    ///
    /// # Returns
    /// The configuration key.
    #[must_use]
    pub fn new(
        scope: StyleScopeKind,
        scope_id: impl Into<String>,
        projection_kind: ProjectionKind,
    ) -> Self {
        Self {
            scope,
            scope_id: scope_id.into(),
            projection_kind,
        }
    }
}

/// One stored style setting with its optimistic revision.
///
/// # Public members
/// - `value`: Stored style input, not yet normalized or merged.
/// - `revision`: Monotonic revision for compare-and-swap updates.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredStyle {
    /// Stored style input, not yet normalized or merged.
    pub value: StyleValue,
    /// Monotonic revision for compare-and-swap updates.
    pub revision: u64,
}

/// Persists style configuration independently from canonical semantic snapshots.
///
/// Style configuration is presentation and policy state. Writing it must never
/// create a canonical source-code migration, so it is kept outside the project
/// snapshot and outside session history.
pub trait ProjectionStyleStore: Send + Sync {
    /// Reads one stored style setting.
    ///
    /// - `key`: Configuration key to read.
    ///
    /// # Returns
    /// The stored setting when one exists.
    ///
    /// # Errors
    /// Returns an application storage error when the store cannot be read.
    fn read(&self, key: &StyleEntryKey) -> Result<Option<StoredStyle>, ApplicationError>;

    /// Writes one setting only when the caller's revision is still current.
    ///
    /// - `key`: Configuration key to write.
    /// - `expected_revision`: Revision observed by the caller, or `None` to write
    ///   without a precondition.
    /// - `value`: New style input.
    ///
    /// # Returns
    /// The authoritative stored setting after the write.
    ///
    /// # Errors
    /// Returns `ApplicationError::SemanticConflict` when the revision is stale and
    /// a storage error when the write fails.
    fn compare_and_swap(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
        value: StyleValue,
    ) -> Result<StoredStyle, ApplicationError>;

    /// Removes one setting when it still matches the caller's revision.
    ///
    /// - `key`: Configuration key to remove.
    /// - `expected_revision`: Revision observed by the caller, or `None` to clear
    ///   without a precondition.
    ///
    /// # Returns
    /// Unit when the setting is absent or was removed.
    ///
    /// # Errors
    /// Returns `ApplicationError::SemanticConflict` when the revision is stale.
    fn clear(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
    ) -> Result<(), ApplicationError>;
}

/// Private setting map backing the in-memory projection style store.
///
/// # Public members
/// - None.
///
/// # Private members
/// - `entries`: Stored settings keyed by scope, scope identity, and kind.
#[derive(Default)]
struct InMemoryProjectionStyleStoreInner {
    /// Stored settings keyed by scope and projection kind.
    entries: BTreeMap<(StyleScopeKind, String, String), StoredStyle>,
}

/// Shared in-memory style configuration store.
///
/// # Public members
/// - `default`: Builds an empty store.
///
/// # Private members
/// - `inner`: Mutex-protected setting map.
#[derive(Clone, Default)]
pub struct InMemoryProjectionStyleStore {
    /// Mutex-protected setting map.
    inner: std::sync::Arc<Mutex<InMemoryProjectionStyleStoreInner>>,
}

impl InMemoryProjectionStyleStore {
    /// Creates an empty in-memory store.
    ///
    /// # Returns
    /// The empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProjectionStyleStore for InMemoryProjectionStyleStore {
    /// Reads one stored style setting.
    ///
    /// - `key`: Configuration key to read.
    ///
    /// # Returns
    /// The stored setting when one exists.
    ///
    /// # Errors
    /// Returns an internal error when the store is poisoned.
    fn read(&self, key: &StyleEntryKey) -> Result<Option<StoredStyle>, ApplicationError> {
        let guard = self.inner.lock().map_err(|error| poisoned(&error))?;
        Ok(guard.entries.get(&map_key(key)).cloned())
    }

    /// Writes one setting with optional revision precondition.
    ///
    /// - `key`: Configuration key to write.
    /// - `expected_revision`: Revision observed by the caller.
    /// - `value`: New style input.
    ///
    /// # Returns
    /// The authoritative stored setting after the write.
    ///
    /// # Errors
    /// Returns a semantic conflict when the revision is stale, or an internal
    /// error when the store is poisoned.
    fn compare_and_swap(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
        value: StyleValue,
    ) -> Result<StoredStyle, ApplicationError> {
        let mut guard = self.inner.lock().map_err(|error| poisoned(&error))?;
        let stored = guard.entries.get(&map_key(key)).cloned();
        if let Some(expected) = expected_revision {
            let current = stored.as_ref().map_or(0, |entry| entry.revision);
            if current != expected {
                return Err(ApplicationError::SemanticConflict {
                    subject: describe(key),
                });
            }
        }
        let revision = stored.map_or(1, |entry| entry.revision + 1);
        let stored = StoredStyle { value, revision };
        guard.entries.insert(map_key(key), stored.clone());
        Ok(stored)
    }

    /// Removes one setting with optional revision precondition.
    ///
    /// - `key`: Configuration key to remove.
    /// - `expected_revision`: Revision observed by the caller.
    ///
    /// # Returns
    /// Unit when the setting is absent or was removed.
    ///
    /// # Errors
    /// Returns a semantic conflict when the revision is stale, or an internal
    /// error when the store is poisoned.
    fn clear(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.inner.lock().map_err(|error| poisoned(&error))?;
        let stored = guard.entries.get(&map_key(key)).cloned();
        if let Some(expected) = expected_revision {
            let current = stored.as_ref().map_or(0, |entry| entry.revision);
            if current != expected {
                return Err(ApplicationError::SemanticConflict {
                    subject: describe(key),
                });
            }
        }
        guard.entries.remove(&map_key(key));
        Ok(())
    }
}

/// Maps one configuration key into the store's ordered map key.
///
/// - `key`: Configuration key to map.
///
/// # Returns
/// The map key tuple using the projection kind's stable label.
fn map_key(key: &StyleEntryKey) -> (StyleScopeKind, String, String) {
    (
        key.scope,
        key.scope_id.clone(),
        key.projection_kind.as_str().to_owned(),
    )
}

/// Describes one configuration key for conflict diagnostics.
///
/// - `key`: Configuration key to describe.
///
/// # Returns
/// A stable safe description.
fn describe(key: &StyleEntryKey) -> String {
    format!(
        "projection style {}:{}:{}",
        key.scope,
        key.scope_id,
        key.projection_kind.as_str()
    )
}

/// Converts a poisoned store lock into an internal application error.
///
/// - `error`: Poisoning error from the lock acquisition.
///
/// # Returns
/// The internal application error carrying the lock diagnostic.
fn poisoned(
    error: &std::sync::PoisonError<std::sync::MutexGuard<'_, InMemoryProjectionStyleStoreInner>>,
) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("projection style store lock poisoned: {error}"),
    }
}

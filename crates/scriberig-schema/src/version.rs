//! Defines transport-safe schema and adapter version metadata.

use std::fmt::{self, Display, Formatter};

use scriberig_core::SnapshotId;

/// Identifies the schema that defines the meaning of canonical domain data.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(pub u32);

impl SchemaVersion {
    /// Creates a schema version from a stable integer.
    ///
    /// # Parameters
    /// - `value`: Explicit schema version.
    ///
    /// # Returns
    /// A schema version value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the numeric schema version.
    ///
    /// # Returns
    /// The stable version number.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for SchemaVersion {
    /// Converts a numeric version into explicit schema metadata.
    ///
    /// # Parameters
    /// - `value`: Numeric schema version.
    ///
    /// # Returns
    /// The corresponding schema version.
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl Display for SchemaVersion {
    /// Formats the numeric schema version.
    ///
    /// # Parameters
    /// - `self`: Version to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// Formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identifies an adapter or producer implementation independently of schema.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdapterMetadata {
    /// Stable adapter or producer identifier.
    pub id: String,
    /// Adapter implementation version.
    pub version: String,
    /// Optional semantics-affecting profile identifier.
    pub profile: Option<String>,
}

/// Classifies whether a migration changes content or schema meaning.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MigrationKind {
    /// Changes ordinary canonical user or imported content.
    Content,
    /// Changes the schema that gives canonical content its meaning.
    Schema,
}

/// Records an explicit schema transition without implementing migration logic.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SchemaMigrationMetadata {
    /// Schema version before the transition.
    pub from: SchemaVersion,
    /// Schema version after the transition.
    pub to: SchemaVersion,
    /// Migration implementation/profile identity.
    pub implementation: AdapterMetadata,
    /// Exact input snapshot, when the migration is snapshot-bound.
    pub input_snapshot: Option<SnapshotId>,
    /// Exact result snapshot, when the migration produced one.
    pub result_snapshot: Option<SnapshotId>,
}

impl SchemaMigrationMetadata {
    /// Creates metadata for an explicit schema transition.
    ///
    /// # Parameters
    /// - `from`: Source schema version.
    /// - `to`: Result schema version.
    /// - `implementation`: Migration implementation metadata.
    ///
    /// # Returns
    /// Schema migration metadata with unbound snapshots.
    #[must_use]
    pub const fn new(
        from: SchemaVersion,
        to: SchemaVersion,
        implementation: AdapterMetadata,
    ) -> Self {
        Self {
            from,
            to,
            implementation,
            input_snapshot: None,
            result_snapshot: None,
        }
    }

    /// Binds exact snapshots to the schema transition.
    ///
    /// # Parameters
    /// - `self`: Metadata to update.
    /// - `input`: Input snapshot.
    /// - `result`: Result snapshot.
    ///
    /// # Returns
    /// Metadata with exact snapshot references.
    #[must_use]
    pub const fn with_snapshots(mut self, input: SnapshotId, result: SnapshotId) -> Self {
        self.input_snapshot = Some(input);
        self.result_snapshot = Some(result);
        self
    }
}

impl AdapterMetadata {
    /// Creates adapter metadata without a profile.
    ///
    /// # Parameters
    /// - `id`: Stable adapter identifier.
    /// - `version`: Adapter implementation version.
    ///
    /// # Returns
    /// Adapter metadata with no profile binding.
    #[must_use]
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            profile: None,
        }
    }

    /// Binds a semantics-affecting profile to the adapter metadata.
    ///
    /// # Parameters
    /// - `self`: Metadata to update.
    /// - `profile`: Stable profile identifier.
    ///
    /// # Returns
    /// Metadata with the profile attached.
    #[must_use]
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterMetadata, MigrationKind, SchemaMigrationMetadata, SchemaVersion};

    /// Verifies schema and adapter versions remain distinct metadata.
    #[test]
    fn schema_and_adapter_versions_are_independent() {
        let schema = SchemaVersion::new(2);
        let adapter = AdapterMetadata::new("markdown-parser", "4").with_profile("tables-v2");
        assert_eq!(schema.value(), 2);
        assert_eq!(adapter.version, "4");
        assert_ne!(schema.to_string(), adapter.version);
    }

    /// Verifies schema migration metadata identifies an explicit transition.
    #[test]
    fn schema_migration_metadata_preserves_transition_kind() {
        let metadata = SchemaMigrationMetadata::new(
            SchemaVersion::new(1),
            SchemaVersion::new(2),
            AdapterMetadata::new("schema-migrator", "1"),
        );
        assert_eq!(metadata.from, SchemaVersion::new(1));
        assert_eq!(metadata.to, SchemaVersion::new(2));
        assert_eq!(MigrationKind::Schema, MigrationKind::Schema);
    }
}

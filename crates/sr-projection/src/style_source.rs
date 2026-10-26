//! Names the identified sources a resolved projection style came from.

use scriberig_core::RevisionId;

use crate::style::{ResolvedStyle, StyleValue};

use std::fmt;

/// Classifies where one style input came from.
///
/// Source classes let resolution prove which inputs applied to a rendered
/// representation without exposing the storage that produced them. Provenance
/// carries style inputs only; it must never hold credentials, unrelated user
/// configuration, or unstable environment data.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum StyleSourceKind {
    /// Value supplied by the domain adapter itself.
    DomainDefault,
    /// Value inferred from repository conventions such as editor configuration.
    RepositoryConvention,
    /// Value declared as shared project policy.
    ProjectPolicy,
    /// Value selected for workspace materialization of managed files.
    WorkspaceMaterializationPolicy,
    /// Value selected by one principal for their own views.
    UserPreference,
    /// Value selected for one specific view session.
    ViewOverride,
    /// Value named explicitly by a caller performing an export.
    ExplicitExportOverride,
}

impl fmt::Display for StyleSourceKind {
    /// Formats the source class as a stable lower-case label.
    ///
    /// - `self`: Source class to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::DomainDefault => "domain-default",
            Self::RepositoryConvention => "repository-convention",
            Self::ProjectPolicy => "project-policy",
            Self::WorkspaceMaterializationPolicy => "workspace-materialization-policy",
            Self::UserPreference => "user-preference",
            Self::ViewOverride => "view-override",
            Self::ExplicitExportOverride => "explicit-export-override",
        };
        formatter.write_str(label)
    }
}

/// One identified style input considered by resolution.
///
/// # Public members
/// - `kind`: Source class this input came from.
/// - `identity`: Stable caller-owned name such as a configuration document id.
/// - `revision`: Exact revision when the source is revisioned knowledge.
/// - `generation`: Exact mutable-source generation when the source is not semantic knowledge.
/// - `value`: Exact style input contributed by this source.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StyleSource {
    /// Source class this input came from.
    pub kind: StyleSourceKind,
    /// Stable caller-owned identity of the contributing source.
    pub identity: String,
    /// Exact revision when the source is revisioned canonical knowledge.
    pub revision: Option<RevisionId>,
    /// Exact generation for a mutable identified source such as a stored setting.
    pub generation: Option<u64>,
    /// Exact style input contributed by this source.
    pub value: StyleValue,
}

impl StyleSource {
    /// Creates one identified style input.
    ///
    /// - `kind`: Source class producing the input.
    /// - `identity`: Stable caller-owned source name.
    /// - `revision`: Exact revision when the source is revisioned knowledge.
    /// - `value`: Exact style input.
    ///
    /// # Returns
    /// The style source record.
    #[must_use]
    pub fn new(
        kind: StyleSourceKind,
        identity: impl Into<String>,
        revision: Option<RevisionId>,
        value: StyleValue,
    ) -> Self {
        Self {
            kind,
            identity: identity.into(),
            revision,
            generation: None,
            value,
        }
    }

    /// Attaches the exact generation of a mutable style source.
    ///
    /// - `self`: Source record to update.
    /// - `generation`: Exact source generation observed during resolution.
    ///
    /// # Returns
    /// The source record with its generation attached.
    #[must_use]
    pub fn with_generation(mut self, generation: u64) -> Self {
        self.generation = Some(generation);
        self
    }
}

/// Records which property of a resolved style one source decided.
///
/// # Public members
/// - `property`: Adapter-owned property key.
/// - `winner`: Index into the ordered applied sources.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StylePropertyOutcome {
    /// Adapter-owned property key.
    pub property: String,
    /// Index into the ordered applied sources that decided this property.
    pub winner: usize,
}

/// Explains how one exact resolved style was derived.
///
/// Provenance is inspectable derived/configuration evidence. It is not canonical
/// semantic content and must not be treated as source-code knowledge.
///
/// # Public members
/// - `resolved`: Exact style a renderer consumed.
/// - `applied`: Sources in the exact order resolution applied them.
/// - `property_outcomes`: Optional per-property winning source records.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResolvedStyleProvenance {
    /// Exact style a renderer consumed.
    pub resolved: ResolvedStyle,
    /// Sources in the exact order resolution applied them.
    pub applied: Vec<StyleSource>,
    /// Optional per-property winning source records.
    pub property_outcomes: Vec<StylePropertyOutcome>,
}

impl ResolvedStyleProvenance {
    /// Records an ordered chain of applied sources for one resolved style.
    ///
    /// - `resolved`: Exact style produced by resolution.
    /// - `applied`: Sources in exact application order, never empty.
    ///
    /// # Returns
    /// The provenance record with no per-property outcomes.
    #[must_use]
    pub fn new(resolved: ResolvedStyle, applied: Vec<StyleSource>) -> Self {
        Self {
            resolved,
            applied,
            property_outcomes: Vec::new(),
        }
    }

    /// Reports whether one source class contributed to this result.
    ///
    /// - `self`: Provenance to inspect.
    /// - `kind`: Source class to look for.
    ///
    /// True when a source of that class was applied.
    ///
    /// # Returns
    /// Whether the applied chain contains the class.
    #[must_use]
    pub fn applied_from(&self, kind: StyleSourceKind) -> bool {
        self.applied.iter().any(|source| source.kind == kind)
    }

    /// Returns the applied source that decided one property.
    ///
    /// - `self`: Provenance to inspect.
    /// - `property`: Adapter-owned property key.
    ///
    /// # Returns
    /// The winning source when per-property provenance recorded it.
    #[must_use]
    pub fn winner_for(&self, property: &str) -> Option<&StyleSource> {
        self.property_outcomes
            .iter()
            .find(|outcome| outcome.property == property)
            .and_then(|outcome| self.applied.get(outcome.winner))
    }
}

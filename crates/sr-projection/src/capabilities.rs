//! Defines explicit and validated projection capabilities.

use std::collections::BTreeSet;

/// Identifies one operation or preservation guarantee of an adapter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ProjectionFeature {
    /// Semantic state can be rendered.
    Renderable,
    /// An external representation can be parsed.
    Parseable,
    /// External edits may be imported.
    Editable,
    /// Parsing and rendering preserve semantic meaning.
    RoundTripCapable,
    /// Reconciliation can retain structure across edits.
    StructurePreserving,
    /// Rendering can preserve source style choices.
    StylePreserving,
}

/// Declares supported projection operations and preservation guarantees.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionCapabilities {
    features: BTreeSet<ProjectionFeature>,
}

/// Errors raised when capability declarations contradict prerequisites.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    /// Editing requires parsing external representations.
    EditableRequiresParseable,
    /// Round-trip capability requires both parsing and rendering.
    RoundTripRequiresParseAndRender,
}

// Construct and inspect capability declarations.
impl ProjectionCapabilities {
    /// Creates a capability declaration after validating its implications.
    ///
    /// # Parameters
    /// - `features`: Operations and guarantees supported by the adapter.
    ///
    /// # Returns
    /// A valid capability declaration.
    ///
    /// # Errors
    /// Returns an error when editing or round-trip guarantees lack prerequisites.
    pub fn new(
        features: impl IntoIterator<Item = ProjectionFeature>,
    ) -> Result<Self, CapabilityError> {
        let features: BTreeSet<_> = features.into_iter().collect();
        if features.contains(&ProjectionFeature::Editable)
            && !features.contains(&ProjectionFeature::Parseable)
        {
            return Err(CapabilityError::EditableRequiresParseable);
        }
        if features.contains(&ProjectionFeature::RoundTripCapable)
            && !(features.contains(&ProjectionFeature::Renderable)
                && features.contains(&ProjectionFeature::Parseable))
        {
            return Err(CapabilityError::RoundTripRequiresParseAndRender);
        }
        Ok(Self { features })
    }

    /// Reports whether a projection feature is supported.
    ///
    /// # Parameters
    /// - `self`: Capability declaration to inspect.
    /// - `feature`: Feature to look up.
    ///
    /// # Returns
    /// `true` when the feature is present.
    #[must_use]
    pub fn supports(&self, feature: ProjectionFeature) -> bool {
        self.features.contains(&feature)
    }

    /// Creates a render-only capability declaration.
    ///
    /// # Parameters
    /// - `self`: No existing value is required.
    ///
    /// # Returns
    /// Capabilities for read-only rendering.
    #[must_use]
    pub fn read_only() -> Self {
        Self {
            features: BTreeSet::from([ProjectionFeature::Renderable]),
        }
    }

    /// Creates capabilities for an editable, renderable projection.
    ///
    /// # Parameters
    /// - `self`: No existing value is required.
    ///
    /// # Returns
    /// Capabilities with parsing and editing enabled.
    #[must_use]
    pub fn editable() -> Self {
        Self {
            features: BTreeSet::from([
                ProjectionFeature::Renderable,
                ProjectionFeature::Parseable,
                ProjectionFeature::Editable,
                ProjectionFeature::RoundTripCapable,
                ProjectionFeature::StructurePreserving,
                ProjectionFeature::StylePreserving,
            ]),
        }
    }
}

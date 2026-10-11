//! Defines explicit and validated projection capabilities.

/// Declares the operations and preservation guarantees an adapter supports.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct ProjectionCapabilities {
    /// Whether semantic state can be rendered.
    pub renderable: bool,
    /// Whether an external representation can be parsed.
    pub parseable: bool,
    /// Whether external edits may be imported.
    pub editable: bool,
    /// Whether parse and render can preserve semantic meaning.
    pub round_trip_capable: bool,
    /// Whether reconciliation can retain structure across edits.
    pub structure_preserving: bool,
    /// Whether rendering can preserve source style choices.
    pub style_preserving: bool,
}

/// Errors raised when a capability declaration contradicts itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    /// Editing requires parsing external representations.
    EditableRequiresParseable,
    /// Round-trip capability requires both parsing and rendering.
    RoundTripRequiresParseAndRender,
}

// Construct capabilities through explicit validation.
impl ProjectionCapabilities {
    /// Creates a capability declaration after validating its implications.
    ///
    /// # Parameters
    /// - `renderable`: Whether rendering is supported.
    /// - `parseable`: Whether parsing is supported.
    /// - `editable`: Whether edits can be imported.
    /// - `round_trip_capable`: Whether semantic round trips are supported.
    /// - `structure_preserving`: Whether structure can be preserved.
    /// - `style_preserving`: Whether style can be preserved.
    ///
    /// # Returns
    /// A valid capability declaration.
    ///
    /// # Errors
    /// Returns an error when editing or round-trip guarantees lack prerequisites.
    pub fn new(
        renderable: bool,
        parseable: bool,
        editable: bool,
        round_trip_capable: bool,
        structure_preserving: bool,
        style_preserving: bool,
    ) -> Result<Self, CapabilityError> {
        if editable && !parseable {
            return Err(CapabilityError::EditableRequiresParseable);
        }
        if round_trip_capable && !(renderable && parseable) {
            return Err(CapabilityError::RoundTripRequiresParseAndRender);
        }
        Ok(Self {
            renderable,
            parseable,
            editable,
            round_trip_capable,
            structure_preserving,
            style_preserving,
        })
    }

    /// Creates a render-only capability declaration.
    ///
    /// # Parameters
    /// - `self`: No existing value is required.
    ///
    /// # Returns
    /// Capabilities for read-only rendering.
    #[must_use]
    pub const fn read_only() -> Self {
        Self {
            renderable: true,
            parseable: false,
            editable: false,
            round_trip_capable: false,
            structure_preserving: false,
            style_preserving: false,
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
    pub const fn editable() -> Self {
        Self {
            renderable: true,
            parseable: true,
            editable: true,
            round_trip_capable: true,
            structure_preserving: true,
            style_preserving: true,
        }
    }
}

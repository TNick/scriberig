//! Carries adapter-determined editability for one exact resolved projection.

use std::fmt;

/// States how much canonical semantics one resolved representation preserves.
///
/// Fidelity is a property of an exact resolved style plus projection kind, not
/// of the projection kind alone. The same semantic entity may project
/// losslessly under one style and be unsafe to edit under another, so callers
/// must ask for fidelity after resolution instead of reading a permanent flag
/// off a specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectionFidelity {
    /// Every canonical category the kind supports is present and editable.
    LosslessEditable,
    /// Some canonical category is absent or reformatted, and the adapter can
    /// still reconcile edits safely because it can distinguish style-hidden
    /// content from user removal.
    LossAwareEditable,
    /// Editing this representation could destroy canonical semantics, so the
    /// representation is presented read-only.
    ReadOnlyLossy,
}

impl ProjectionFidelity {
    /// Reports whether an edit session may open on this representation.
    ///
    /// - `self`: Fidelity being consulted.
    ///
    /// False only for read-only lossy output.
    #[must_use]
    /// # Returns
    /// Whether the representation may be edited.
    pub fn is_editable(self) -> bool {
        !matches!(self, Self::ReadOnlyLossy)
    }

    /// Reports whether reconciliation must be loss-aware.
    ///
    /// - `self`: Fidelity being consulted.
    ///
    /// True when canonical content may be absent from the representation.
    #[must_use]
    /// # Returns
    /// Whether reconciliation requires loss awareness.
    pub fn requires_loss_awareness(self) -> bool {
        matches!(self, Self::LossAwareEditable)
    }
}

impl fmt::Display for ProjectionFidelity {
    /// Formats the fidelity as a stable lower-case label.
    ///
    /// - `self`: Fidelity to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::LosslessEditable => "lossless-editable",
            Self::LossAwareEditable => "loss-aware-editable",
            Self::ReadOnlyLossy => "read-only-lossy",
        };
        formatter.write_str(label)
    }
}

/// Reports fidelity for one resolved style together with adapter evidence.
///
/// # Public members
/// - `fidelity`: Editability classification for the exact resolved style.
/// - `omitted_categories`: Adapter-owned names of canonical categories the
///   representation does not carry, empty when nothing is omitted.
/// - `explanation`: Optional adapter-owned reason safe for client display.
///
/// # Private members
/// - None.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FidelityAssessment {
    /// Editability classification for the exact resolved style.
    pub fidelity: ProjectionFidelity,
    /// Adapter-owned canonical categories absent from the representation.
    pub omitted_categories: Vec<String>,
    /// Optional adapter-owned explanation safe for client presentation.
    pub explanation: Option<String>,
}

impl FidelityAssessment {
    /// Builds an assessment for a representation carrying full semantics.
    ///
    /// # Returns
    /// A lossless editable assessment.
    #[must_use]
    pub fn lossless() -> Self {
        Self {
            fidelity: ProjectionFidelity::LosslessEditable,
            omitted_categories: Vec::new(),
            explanation: None,
        }
    }

    /// Builds an assessment for a representation that hides known categories.
    ///
    /// - `omitted_categories`: Adapter-owned categories absent from output.
    /// - `explanation`: Optional adapter-owned reason for clients.
    ///
    /// # Returns
    /// A loss-aware editable assessment.
    #[must_use]
    pub fn loss_aware(omitted_categories: Vec<String>, explanation: Option<String>) -> Self {
        Self {
            fidelity: ProjectionFidelity::LossAwareEditable,
            omitted_categories,
            explanation,
        }
    }

    /// Builds a conservative assessment for an adapter that cannot classify.
    ///
    /// - `explanation`: Adapter-owned reason for the read-only classification.
    ///
    /// Unmigrated adapters must resolve to this so a surface never opens an
    /// edit session on representation it has not proven safe to reconcile.
    ///
    /// # Returns
    /// A read-only lossy assessment.
    #[must_use]
    pub fn conservative(explanation: impl Into<String>) -> Self {
        Self {
            fidelity: ProjectionFidelity::ReadOnlyLossy,
            omitted_categories: Vec::new(),
            explanation: Some(explanation.into()),
        }
    }
}

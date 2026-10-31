//! Defines the domain-owned extension seam for projection style meaning.

use crate::adapter::ProjectionResult;
use crate::diagnostic::{
    DiagnosticDetail, DiagnosticKind, DiagnosticSource, DiagnosticStage, ProjectionDiagnostic,
};
use crate::fidelity::FidelityAssessment;
use crate::model::ProjectionKind;
use crate::purpose::ProjectionPurpose;
use crate::style::{ResolvedStyle, StyleValue};
use crate::style_schema::StyleSchema;

/// Reports the adapter-owned style meaning behind an otherwise opaque value.
///
/// `sr-projection` stays domain-neutral: it never interprets a style property.
/// A domain implements this trait to declare its schema, normalize and merge
/// canonical style inputs, provide exact defaults, and classify fidelity for an
/// exact resolved style. An adapter that implements nothing keeps working: it
/// resolves to one fixed default style and a conservative read-only fidelity,
/// which is the documented migration position for unmigrated adapters.
///
/// # Public members
/// - `style_schema`: Declares the adapter's schema for one projection kind.
/// - `default_style`: Provides the exact default style for one kind.
/// - `normalize`: Converts a style input to canonical exact bytes.
/// - `merge`: Combines two style inputs with domain-defined rules.
/// - `classify`: Determines fidelity for one exact resolved style and purpose.
///
/// # Private members
/// - None.
pub trait ProjectionStyleAdapter: Send + Sync {
    /// Returns the adapter-owned style schema for one projection kind.
    ///
    /// - `self`: Adapter declaring its schema.
    /// - `kind`: Projection kind being inspected.
    ///
    /// Structured schema metadata for client presentation, or `None` when this
    /// adapter exposes no style discovery yet.
    ///
    /// # Returns
    /// The adapter schema for the kind when one is declared.
    fn style_schema(&self, kind: &ProjectionKind) -> Option<StyleSchema> {
        let _ = kind;
        None
    }

    /// Returns the exact default style for one projection kind.
    ///
    /// - `self`: Adapter owning the default.
    /// - `kind`: Projection kind being rendered.
    ///
    /// The canonical default style, or a structured diagnostic when the adapter
    /// has not declared one.
    ///
    /// # Errors
    /// Returns a structured diagnostic when no default style exists.
    ///
    /// # Returns
    /// The exact default style for the kind.
    fn default_style(&self, kind: &ProjectionKind) -> ProjectionResult<ResolvedStyle> {
        Err(Box::new(unsupported_style_diagnostic(
            kind,
            "no-default-style",
            "the projection adapter declares no default projection style",
        )))
    }

    /// Converts one style input into canonical exact bytes.
    ///
    /// - `self`: Adapter owning normalization.
    /// - `kind`: Projection kind the style will render.
    /// - `input`: Style input to normalize.
    ///
    /// Canonical style bytes, or a structured diagnostic naming the invalid
    /// property. Normalization must be deterministic so identical semantics
    /// produce identical render and cache identity.
    ///
    /// # Errors
    /// Returns a structured diagnostic for invalid or unsupported style input.
    ///
    /// # Returns
    /// The canonical normalized style value.
    fn normalize(&self, kind: &ProjectionKind, input: &StyleValue) -> ProjectionResult<StyleValue> {
        let _ = input;
        Err(Box::new(unsupported_style_diagnostic(
            kind,
            "no-style-normalization",
            "the projection adapter declares no style normalization",
        )))
    }

    /// Combines two canonical style inputs with domain-defined rules.
    ///
    /// - `self`: Adapter owning merge semantics.
    /// - `kind`: Projection kind the style will render.
    /// - `base`: Lower-precedence canonical style.
    /// - `overlay`: Higher-precedence canonical style.
    ///
    /// The canonical merged style, or a structured diagnostic when the domain
    /// forbids the combination. Merge is domain meaning, never a generic object
    /// overlay, and required materialization properties may not be dropped.
    ///
    /// # Errors
    /// Returns a structured diagnostic when the combination is invalid.
    ///
    /// # Returns
    /// The canonical merged style value.
    fn merge(
        &self,
        kind: &ProjectionKind,
        base: &StyleValue,
        overlay: &StyleValue,
    ) -> ProjectionResult<StyleValue> {
        let _ = (base, overlay);
        Err(Box::new(unsupported_style_diagnostic(
            kind,
            "no-style-merge",
            "the projection adapter declares no style merge semantics",
        )))
    }

    /// Classifies editability for one exact resolved style and purpose.
    ///
    /// - `self`: Adapter owning fidelity.
    /// - `kind`: Projection kind being rendered.
    /// - `style`: Exact resolved style the renderer will consume.
    /// - `purpose`: Why the projection is being produced.
    ///
    /// Fidelity for that exact style. Adapters that have not proven loss-aware
    /// reconciliation must return a conservative read-only assessment.
    ///
    /// # Returns
    /// The fidelity assessment for the exact resolved style.
    #[must_use]
    fn classify(
        &self,
        kind: &ProjectionKind,
        style: &ResolvedStyle,
        purpose: ProjectionPurpose,
    ) -> FidelityAssessment {
        let _ = (style, purpose);
        FidelityAssessment::conservative(format!(
            "adapter for projection kind '{}' does not classify style fidelity",
            kind.as_str()
        ))
    }
}

/// Builds a structured diagnostic for an unsupported style operation.
///
/// - `kind`: Projection kind whose adapter lacks the style capability.
/// - `code`: Stable machine-readable diagnostic code.
/// - `message`: Human-readable explanation safe for clients.
///
/// # Returns
/// A non-retryable configuration diagnostic at the style resolution stage.
#[must_use]
pub fn unsupported_style_diagnostic(
    kind: &ProjectionKind,
    code: &str,
    message: &str,
) -> ProjectionDiagnostic {
    ProjectionDiagnostic {
        kind: DiagnosticKind::UnsupportedOperation,
        stage: DiagnosticStage::StyleResolution,
        projection: None,
        scope: None,
        detail: DiagnosticDetail {
            code: code.to_owned(),
            data: kind.as_str().as_bytes().to_vec(),
        },
        source: Some(DiagnosticSource {
            subsystem: "projection-style-adapter".to_owned(),
            code: code.to_owned(),
            message: None,
        }),
        message: Some(message.to_owned()),
        retryable: false,
    }
}

/// Builds a structured diagnostic for an invalid style combination.
///
/// - `kind`: Projection kind whose domain rejected the combination.
/// - `code`: Stable machine-readable diagnostic code.
/// - `message`: Human-readable explanation safe for clients.
/// - `data`: Adapter-owned structured bytes naming the offending property.
///
/// # Returns
/// A non-retryable invalid-configuration diagnostic at the style stage.
#[must_use]
pub fn invalid_style_diagnostic(
    kind: &ProjectionKind,
    code: &str,
    message: &str,
    data: Vec<u8>,
) -> ProjectionDiagnostic {
    let mut diagnostic = unsupported_style_diagnostic(kind, code, message);
    diagnostic.kind = DiagnosticKind::InvalidConfiguration;
    diagnostic.detail.data = data;
    diagnostic
}

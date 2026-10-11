//! Builds Markdown projection diagnostics.

use super::{DiagnosticKind, DiagnosticStage, ProjectionDiagnostic};

/// Builds one generic projection diagnostic.
/// - `super`: Parent-module imports used by this helper.
/// - `stage`: Lifecycle stage.
/// - `kind`: Diagnostic category.
/// - `code`: Stable diagnostic code.
/// - `message`: Human-readable message.
///
/// # Returns
/// Structured projection diagnostic.
pub(super) fn diagnostic(
    stage: DiagnosticStage,
    kind: DiagnosticKind,
    code: &str,
    message: impl Into<String>,
) -> ProjectionDiagnostic {
    let message = message.into();
    ProjectionDiagnostic {
        kind,
        stage,
        projection: None,
        scope: None,
        detail: sr_projection::DiagnosticDetail {
            code: code.to_owned(),
            data: Vec::new(),
        },
        source: Some(sr_projection::DiagnosticSource {
            subsystem: "sr-markdown".to_owned(),
            code: code.to_owned(),
            message: Some(message.clone()),
        }),
        message: Some(message),
        retryable: false,
    }
}

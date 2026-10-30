//! Builds safe candidate errors for CAD edits against exact DXF sources.

use scriberig_core::EntityId;

use crate::ProjectError;

/// Creates a safe exact-base DXF candidate error.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe explanatory text.
///
/// # Returns
/// Structured project operation error without source bytes.
pub(crate) fn dxf_candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}

/// Creates a safe DXF operation error linked to its semantic target.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe explanatory text.
/// - `path`: Project-relative DXF path containing the target.
/// - `entity`: Semantic entity targeted by the rejected operation.
///
/// # Returns
/// Structured project error retaining the exact target entity identity.
pub(crate) fn dxf_candidate_entity_error(
    code: &'static str,
    message: &str,
    path: &str,
    entity: EntityId,
) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: vec![crate::ImportDiagnostic {
            entity: Some(entity),
            path: path.to_owned(),
            adapter: "dxf".to_owned(),
            stage: "edit".to_owned(),
            code: code.to_owned(),
            message: message.to_owned(),
            retryable: false,
        }],
    }
}

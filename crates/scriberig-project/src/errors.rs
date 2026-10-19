//! Defines structured errors used by project scanning and importing.

use std::fmt::Display;

use scriberig_core::ProjectId;
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{ImportDiagnostic, ProjectError};

/// Converts a storage failure into a project error.
///
/// - `error`: Storage failure.
/// - `super`: Parent module visibility marker.
///
/// A structured project error.
/// # Returns
/// The operation result.
pub(super) fn storage_error(error: impl Display) -> ProjectError {
    ProjectError {
        code: "storage_error",
        message: error.to_string(),
        diagnostics: Vec::new(),
    }
}

/// Converts a debug-only store failure into a project error.
///
/// - `error`: Store failure with no `Display` implementation.
/// - `super`: Parent module containing project error conversion.
///
/// A structured storage error.
/// # Returns
/// The operation result.
pub(super) fn storage_debug_error(error: impl std::fmt::Debug) -> ProjectError {
    ProjectError {
        code: "storage_error",
        message: format!("{error:?}"),
        diagnostics: Vec::new(),
    }
}

/// Converts a filesystem failure into a project error.
///
/// - `error`: Filesystem failure.
/// - `super`: Parent module visibility marker.
///
/// A structured project error.
/// # Returns
/// The operation result.
pub(super) fn filesystem_error(error: &std::io::Error) -> ProjectError {
    ProjectError {
        code: "filesystem_error",
        message: error.to_string(),
        diagnostics: Vec::new(),
    }
}

/// Converts a Git selection failure into a stable project error.
///
/// - `error`: Git command or repository failure.
/// - `super`: Parent module visibility marker.
///
/// A structured Git repository requirement error.
/// # Returns
/// The operation result.
pub(super) fn git_error(error: &std::io::Error) -> ProjectError {
    ProjectError {
        code: "git_repository_required",
        message: error.to_string(),
        diagnostics: Vec::new(),
    }
}

/// Creates a missing-project error.
///
/// - `project`: Missing project identity.
/// - `super`: Parent module visibility marker.
///
/// A structured project error.
/// # Returns
/// The operation result.
pub(super) fn missing_project(project: ProjectId) -> ProjectError {
    ProjectError {
        code: "project_not_found",
        message: project.to_string(),
        diagnostics: Vec::new(),
    }
}

/// Creates a snapshot promotion error.
///
/// - `message`: Promotion failure detail.
/// - `super`: Parent module visibility marker.
///
/// A structured project error.
/// # Returns
/// The operation result.
pub(super) fn promotion_error(message: String) -> ProjectError {
    ProjectError {
        code: "promotion_failed",
        message,
        diagnostics: Vec::new(),
    }
}

/// Creates an adapter error from parser or extractor diagnostics.
///
/// - `file`: Source file that failed validation.
/// - `stage`: Import stage that reported the diagnostics.
/// - `code`: Stable diagnostic code.
/// - `messages`: Parser or extractor messages.
/// - `retryable`: Whether source repair may make retry succeed.
/// - `crate`: Crate visibility marker.
///
/// A structured project error with all diagnostics preserved.
/// # Returns
/// The operation result.
pub(crate) fn adapter_diagnostics_error(
    file: &ProjectFileRecord,
    stage: &str,
    code: &str,
    messages: &[String],
    retryable: bool,
) -> ProjectError {
    let diagnostics = messages
        .iter()
        .map(|message| ImportDiagnostic {
            path: file.relative_path.clone(),
            adapter: file.adapter.clone(),
            stage: stage.to_owned(),
            code: code.to_owned(),
            message: message.clone(),
            retryable,
        })
        .collect::<Vec<_>>();
    ProjectError {
        code: "import_diagnostics",
        message: format!("{}: {}", file.relative_path, code),
        diagnostics,
    }
}

/// Creates an invalid-input error.
///
/// - `message`: Invalid-input detail.
/// - `crate`: Crate visibility marker.
///
/// A structured project error.
/// # Returns
/// The operation result.
pub(crate) fn invalid_input(message: String) -> ProjectError {
    ProjectError {
        code: "invalid_input",
        message,
        diagnostics: Vec::new(),
    }
}

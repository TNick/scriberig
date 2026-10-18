//! Resolves stable principals for shared presentation settings.

use crate::{ApplicationContext, ApplicationError};

/// Resolves the stable caller identity required for shared preferences.
///
/// # Parameters
/// - `context`: Authorized application context.
///
/// # Returns
/// The stable principal identifier.
///
/// # Errors
/// Returns invalid-request when the caller has no principal identity.
pub(super) fn presentation_principal(
    context: &ApplicationContext,
) -> Result<&str, ApplicationError> {
    context
        .user_preferences_principal
        .as_deref()
        .or_else(|| {
            context
                .principal
                .as_ref()
                .map(|principal| principal.id.as_str())
        })
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: "agent presentation preferences".to_owned(),
            message: "a principal identity is required".to_owned(),
        })
}

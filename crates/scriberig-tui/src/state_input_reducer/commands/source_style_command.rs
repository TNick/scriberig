//! Parses personal Rust source-style preference commands.

/// Builds a shared user-scoped Rust projection style setting request.
///
/// The command carries the style as opaque adapter-owned bytes. The Rust domain
/// validates and canonicalizes its meaning through the shared application path.
///
/// # Parameters
/// - `arguments`: Style bytes to store, or the word `clear`.
///
/// # Returns
/// A typed user preference request for the shared application service.
///
/// # Errors
/// Returns a usage error for empty, oversized, or malformed input.
pub(super) fn preference_request(
    arguments: &str,
) -> Result<scriberig_protocol::ProjectionStyleSettingRequest, &'static str> {
    if arguments.eq_ignore_ascii_case("clear") {
        return Ok(request(None));
    }
    if arguments.is_empty() || arguments.len() > 512 || arguments.chars().any(char::is_control) {
        return Err(
            "Usage: /source-style show, /source-style <Rust style fields>, or /source-style clear.",
        );
    }
    Ok(request(Some(arguments.as_bytes().to_vec())))
}

/// Creates a setting request for the Rust source projection kind.
///
/// # Parameters
/// - `style`: Canonical domain style input or `None` to clear the preference.
///
/// # Returns
/// A user-scoped style setting request without a revision precondition.
fn request(style: Option<Vec<u8>>) -> scriberig_protocol::ProjectionStyleSettingRequest {
    scriberig_protocol::ProjectionStyleSettingRequest {
        scope: scriberig_protocol::ProjectionStyleScope::User,
        projection_kind: "rust".to_owned(),
        expected_revision: None,
        style,
    }
}

#[cfg(test)]
mod tests {
    use super::preference_request;

    /// Leaves field validation to the Rust domain adapter while preserving exact input.
    #[test]
    fn personal_style_request_keeps_adapter_owned_bytes() {
        let request =
            preference_request("indent=tabs,indent-width=2,blank-lines=0").expect("style request");
        assert_eq!(request.projection_kind, "rust");
        assert_eq!(
            request.scope,
            scriberig_protocol::ProjectionStyleScope::User
        );
        assert_eq!(
            request.style.as_deref(),
            Some(b"indent=tabs,indent-width=2,blank-lines=0".as_slice())
        );
        assert!(
            preference_request("clear")
                .expect("clear request")
                .style
                .is_none()
        );
        assert!(preference_request("").is_err());
    }
}

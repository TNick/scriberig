//! Filters untrusted provider response identities for durable provenance.

/// Retains a bounded public provider response identity without control characters.
///
/// - `value`: Provider-supplied response identity.
///
/// A safe identity when it is short, nonempty ASCII using the accepted token alphabet.
/// # Returns
/// `None` for absent or unsafe provider values.
pub(super) fn safe_provider_response_id(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:~/".contains(&byte))
    {
        return None;
    }
    Some(value.to_owned())
}

#[cfg(test)]
mod tests {
    //! Verifies provider response identities remain bounded inert identifiers.

    use super::safe_provider_response_id;

    /// Keeps ordinary provider IDs and rejects control, empty, and overlong values.
    #[test]
    fn provider_response_identity_is_bounded_and_control_free() {
        assert_eq!(
            safe_provider_response_id("resp_abc-123:vendor/model"),
            Some("resp_abc-123:vendor/model".to_owned())
        );
        assert_eq!(safe_provider_response_id(""), None);
        assert_eq!(safe_provider_response_id("response\u{1b}[31m"), None);
        assert_eq!(safe_provider_response_id(&"x".repeat(257)), None);
    }
}

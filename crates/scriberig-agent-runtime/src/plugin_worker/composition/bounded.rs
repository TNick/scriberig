//! Bounds and sanitizes every value that crosses the plugin composition seam.

/// Largest argument payload accepted for one plugin tool call.
pub(super) const MAX_PLUGIN_ARGUMENT_BYTES: usize = 64 * 1024;
/// Largest retained plugin argument payload in durable evidence.
pub(super) const MAX_EVIDENCE_INPUT_BYTES: usize = 256 * 1024;
/// Largest retained plugin result payload in durable evidence.
pub(super) const MAX_EVIDENCE_OUTPUT_BYTES: usize = 12_000;
/// Longest retained safe worker failure message.
pub(super) const MAX_FAILURE_CHARS: usize = 500;
/// Longest accepted plugin tool or capability identifier.
pub(super) const MAX_IDENTIFIER_CHARS: usize = 64;
/// Longest accepted worker-advertised tool description.
pub(super) const MAX_DESCRIPTION_CHARS: usize = 512;

/// Keeps a plugin identifier inside the advertised-tool character set.
///
/// # Parameters
/// - `identifier`: Worker tool name or plugin capability name.
///
/// # Returns
/// `true` for a non-empty identifier of at most `MAX_IDENTIFIER_CHARS`
/// characters using ASCII letters, digits, and `.`/`_`/`-`/`:` separators.
#[must_use]
pub(super) fn is_safe_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.len() <= MAX_IDENTIFIER_CHARS
        && identifier.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | ':')
        })
}

/// Sanitizes and bounds one worker-supplied tool description.
///
/// # Parameters
/// - `description`: Description advertised by the worker.
///
/// # Returns
/// A control-free description of at most `MAX_DESCRIPTION_CHARS` scalar values.
pub(super) fn bounded_description(description: &str) -> String {
    description
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_DESCRIPTION_CHARS)
        .collect()
}

/// Bounds one safe failure message for provider and durable output.
///
/// # Parameters
/// - `message`: Worker or upstream failure text.
///
/// # Returns
/// A control-free message of at most `MAX_FAILURE_CHARS` scalar values.
pub(super) fn bounded_failure(message: &str) -> String {
    message
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_FAILURE_CHARS)
        .collect()
}

/// Bounds retained durable evidence text by Unicode scalar value.
///
/// # Parameters
/// - `text`: Settled tool text.
///
/// # Returns
/// At most `MAX_EVIDENCE_OUTPUT_BYTES` scalar values.
pub(super) fn bounded_retained(text: &str) -> String {
    text.chars().take(MAX_EVIDENCE_OUTPUT_BYTES).collect()
}

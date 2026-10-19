//! Removes exact credential reflections from provider output and streaming deltas.

use crate::provider::ModelResponse;
use zeroize::Zeroizing;

/// Delays only enough text to prevent a credential crossing delta boundaries.
pub(crate) struct DeltaRedactor<'a> {
    /// Current request's credential, borrowed only during transport.
    secret: Option<&'a str>,
    /// Unemitted suffix that may contain a partial credential.
    pending: Zeroizing<String>,
}

impl<'a> DeltaRedactor<'a> {
    /// Creates a per-request streaming redactor.
    ///
    /// - `secret`: Exact current request credential, if present.
    ///
    /// A redactor with an empty zeroizing suffix buffer.
    /// # Returns
    /// The operation result.
    pub(crate) fn new(secret: Option<&'a str>) -> Self {
        Self {
            secret,
            pending: Zeroizing::new(String::new()),
        }
    }

    /// Emits text whose credential matches can no longer cross a future delta.
    ///
    /// - `text`: New provider output fragment.
    ///
    /// A safe output prefix; a short suffix remains buffered.
    /// # Returns
    /// The operation result.
    pub(crate) fn push(&mut self, text: &str) -> String {
        let Some(secret) = self.secret else {
            return text.to_owned();
        };
        self.pending.push_str(text);
        let mut offset = 0;
        let mut output = String::new();
        while self.pending.len() - offset >= secret.len() {
            let suffix = &self.pending[offset..];
            if suffix.starts_with(secret) {
                output.push_str("[REDACTED]");
                offset += secret.len();
            } else if let Some(character) = suffix.chars().next() {
                output.push(character);
                offset += character.len_utf8();
            }
        }
        self.pending.drain(..offset);
        output
    }

    /// Releases the unmatched trailing text after successful stream completion.
    ///
    /// The final suffix, which cannot contain a complete credential.
    /// # Returns
    /// The operation result.
    pub(crate) fn finish(&mut self) -> String {
        std::mem::take(&mut *self.pending)
    }
}

/// Redacts exact credential reflections before normalized output leaves transport.
///
/// - `response`: Model output and metadata to sanitize.
/// - `secret`: Current request credential, if any.
pub(crate) fn redact_response(response: &mut ModelResponse, secret: Option<&str>) {
    let Some(secret) = secret else {
        return;
    };
    for text in [
        &mut response.text,
        &mut response.metadata.request_id,
        &mut response.metadata.model_fingerprint,
    ]
    .into_iter()
    .flatten()
    {
        *text = text.replace(secret, "[REDACTED]");
    }
    for call in &mut response.tool_calls {
        call.id = call.id.replace(secret, "[REDACTED]");
        call.name = call.name.replace(secret, "[REDACTED]");
        redact_json(&mut call.arguments, secret);
    }
}

/// Scrubs credential reflections from structured tool arguments and their keys.
///
/// - `value`: Structured argument value.
/// - `secret`: Exact request credential.
fn redact_json(value: &mut serde_json::Value, secret: &str) {
    match value {
        serde_json::Value::String(text) => *text = text.replace(secret, "[REDACTED]"),
        serde_json::Value::Array(items) => {
            for item in items {
                redact_json(item, secret);
            }
        }
        serde_json::Value::Object(object) => {
            let old = std::mem::take(object);
            for (key, mut item) in old {
                redact_json(&mut item, secret);
                object.insert(key.replace(secret, "[REDACTED]"), item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{DeltaRedactor, redact_json};

    /// Every possible fragment boundary preserves exact credential redaction.
    #[test]
    fn all_fragment_boundaries_are_safe() {
        let text = "prefix CANARY suffix CANARY";
        for boundary in 0..=text.len() {
            let mut redactor = DeltaRedactor::new(Some("CANARY"));
            let mut output = redactor.push(&text[..boundary]);
            output.push_str(&redactor.push(&text[boundary..]));
            output.push_str(&redactor.finish());
            assert_eq!(output, "prefix [REDACTED] suffix [REDACTED]");
        }
    }

    /// Multibyte text and structured keys remain valid after redaction.
    #[test]
    fn unicode_and_structured_values_are_scrubbed() {
        let mut redactor = DeltaRedactor::new(Some("秘密"));
        let mut output = redactor.push("é秘");
        output.push_str(&redactor.push("密 end"));
        output.push_str(&redactor.finish());
        assert_eq!(output, "é[REDACTED] end");
        let mut value = serde_json::json!({"CANARY": ["CANARY", {"value":"CANARY"}]});
        redact_json(&mut value, "CANARY");
        assert!(!value.to_string().contains("CANARY"));
    }
}

//! Recovers source-authored Rust comments without making coordinates identities.

use scriberig_domain_code::{Comment, CommentKind};

/// A comment plus optional source-location metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustComment {
    /// Shared semantic comment value.
    pub value: Comment,
    /// Zero-based source line when available.
    pub line: usize,
}

/// Recovers line and documentation comments from source.
/// - `source`: Rust source text.
///
/// Comments in source order.
#[must_use]
/// # Returns
/// The operation result.
pub fn recover_comments(source: &str) -> Vec<RustComment> {
    source
        .lines()
        .enumerate()
        .filter_map(|(line, text)| {
            let trimmed = text.trim();
            let (kind, body) = if let Some(value) = trimmed.strip_prefix("///") {
                (CommentKind::Documentation, value.trim())
            } else {
                let value = trimmed.strip_prefix("//")?;
                (CommentKind::Ordinary, value.trim())
            };
            Some(RustComment {
                value: Comment {
                    kind,
                    text: body.to_owned(),
                },
                line,
            })
        })
        .collect()
}

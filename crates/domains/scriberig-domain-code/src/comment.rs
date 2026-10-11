//! Defines first-class source comment metadata.
/// Shared comment classification across language adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommentKind {
    /// Ordinary comment.
    Ordinary,
    /// Documentation comment.
    Documentation,
    /// Explanation.
    Explanation,
    /// Invariant statement.
    Invariant,
    /// Safety note.
    Safety,
    /// Todo marker.
    Todo,
    /// Directive.
    Directive,
    /// Generated comment.
    Generated,
}
/// A source comment independent of file coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment {
    /// Comment kind.
    pub kind: CommentKind,
    /// Text content.
    pub text: String,
}

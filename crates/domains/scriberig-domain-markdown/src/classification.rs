//! Classifies Markdown information by canonical ownership.

/// Identifies whether a Markdown field contributes to semantic identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InformationClass {
    /// Changes to this information belong to canonical state.
    Canonical,
    /// This information is formatting detail by default.
    ProjectionOnly,
    /// This information can be recomputed from canonical inputs.
    Derived,
}

/// Returns the class assigned to one named Markdown information group.
///
/// - `name`: Stable information group name.
///
/// The canonical ownership class for the group, or `None` when unknown.
#[must_use]
/// # Returns
/// The operation result.
pub fn classify(name: &str) -> Option<InformationClass> {
    match name {
        "structure" | "text" | "links" | "images" | "inline" => Some(InformationClass::Canonical),
        "indentation" | "line_wrapping" | "blank_lines" | "table_padding" => {
            Some(InformationClass::ProjectionOnly)
        }
        "summary" | "topic_labels" | "outline" | "description" | "search_index" => {
            Some(InformationClass::Derived)
        }
        _ => None,
    }
}

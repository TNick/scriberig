//! Defines canonical and derived code-domain classifications.
/// Classifies whether code knowledge is canonical or derived.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InformationClass {
    /// Source-authored semantic state.
    Canonical,
    /// Computed or generated state.
    Derived,
}
/// Returns the class for a field or artifact kind.
///
/// # Parameters
/// - `name`: Stable field name.
///
/// # Returns
/// Classification when recognized.
#[must_use]
pub fn classify(name: &str) -> Option<InformationClass> {
    if matches!(
        name,
        "declaration" | "parameter" | "statement" | "expression" | "comment" | "code_group"
    ) {
        Some(InformationClass::Canonical)
    } else if matches!(
        name,
        "inferred_type" | "call_graph" | "control_flow" | "description" | "diagnostic"
    ) {
        Some(InformationClass::Derived)
    } else {
        None
    }
}

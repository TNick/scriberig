//! Projects and bounds upstream content blocks for durable request history.

/// Joins provider-visible text while flagging omitted non-text content.
///
/// # Parameters
/// - `content`: Upstream content blocks.
///
/// # Returns
/// Text in original order and whether any non-text block was omitted.
pub(super) fn text_content_with_omissions(content: &[rpi_ai::types::Content]) -> (String, bool) {
    let mut omitted = false;
    let text = content
        .iter()
        .filter_map(|content| {
            if let rpi_ai::types::Content::Text(text) = content {
                Some(text.text.as_str())
            } else {
                omitted = true;
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");
    (text, omitted)
}

/// Joins text from upstream user content blocks.
///
/// # Parameters
/// - `content`: Upstream user content blocks.
///
/// # Returns
/// Text blocks in their original order.
pub(super) fn text_content(content: &[rpi_ai::types::Content]) -> String {
    content
        .iter()
        .filter_map(|content| match content {
            rpi_ai::types::Content::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Copies at most the remaining character budget from one string.
///
/// # Parameters
/// - `value`: Content to copy.
/// - `budget`: Mutable remaining character allowance.
///
/// # Returns
/// The bounded prefix and updated remaining budget.
pub(super) fn take_bounded(value: &str, budget: &mut usize) -> String {
    let bounded = value.chars().take(*budget).collect::<String>();
    *budget -= bounded.chars().count();
    bounded
}

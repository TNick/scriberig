//! Owns terminal presentation for shared application inspection results.

use scriberig_app::InspectResult;

use crate::trust::{InspectDepth, OutputFormat, render_inspection_with_format};

/// Renders an inspection result for a selected terminal representation.
///
/// # Parameters
/// - `result`: Shared application inspection result.
/// - `depth`: Requested trust detail.
/// - `format`: Human or structured output format.
///
/// # Returns
/// Terminal-ready inspection text.
#[must_use]
pub fn render_inspection_output(
    result: &InspectResult,
    depth: InspectDepth,
    format: OutputFormat,
) -> String {
    render_inspection_with_format(result, depth, format)
}

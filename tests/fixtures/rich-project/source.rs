//! Defines the deterministic parcel-area source fixture used by rich-project tests.

/// Computes a rectangular parcel's area for the mixed-project source anchor.
///
/// # Parameters
/// - `width`: Parcel width in the fixture's unit.
/// - `height`: Parcel height in the fixture's unit.
///
/// # Returns
/// The product of the two dimensions.
pub fn parcel_area(width: f64, height: f64) -> f64 {
    width * height
}

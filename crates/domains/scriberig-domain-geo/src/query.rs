//! Provides bounded snapshot-scoped attribute and geometry-envelope queries.

use std::cmp::Ordering;

use scriberig_core::{EntityId, SnapshotId};
use serde::{Deserialize, Serialize};

use crate::{Coordinate, CoordinateReferenceSystem, Feature, GeoValue, Geometry};

/// Selects geospatial features by an exact attribute or bounding envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GeoPredicate {
    /// Matches an attribute whose typed value exactly equals the requested value.
    AttributeEquals {
        /// Authored field name.
        field: String,
        /// Typed value to match.
        value: GeoValue,
    },
    /// Matches feature geometry envelopes overlapping the supplied bounds.
    EnvelopeOverlaps {
        /// Query rectangle with explicit coordinate reference system.
        bounds: GeoBounds,
    },
}

/// Describes an axis-aligned query rectangle in one explicit CRS.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoBounds {
    /// Minimum x/easting coordinate as decimal text.
    pub min_x: String,
    /// Minimum y/northing coordinate as decimal text.
    pub min_y: String,
    /// Maximum x/easting coordinate as decimal text.
    pub max_x: String,
    /// Maximum y/northing coordinate as decimal text.
    pub max_y: String,
    /// Coordinate reference system of these bounds.
    pub crs: CoordinateReferenceSystem,
}

/// Identifies one feature in the exact snapshot used by a query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoFeatureHandle {
    /// Exact feature semantic identity.
    pub feature: EntityId,
    /// Exact feature layer identity.
    pub layer: EntityId,
    /// Snapshot against which the feature was selected.
    pub snapshot: SnapshotId,
}

/// Returns a bounded page of matching semantic feature handles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeoQueryPage {
    /// Matching feature handles in stable semantic identity order.
    pub matches: Vec<GeoFeatureHandle>,
    /// Whether another matching page exists.
    pub has_more: bool,
}

/// Describes invalid bounds, page limits, layer membership, or CRS mismatches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeoQueryError {
    /// The requested page size is zero or above the collection's configured maximum.
    InvalidPageSize,
    /// A feature does not belong to the layer for which an index is being built.
    WrongLayer,
    /// Bounds are not finite, ordered, or represented as valid decimal numbers.
    InvalidBounds,
    /// Query CRS differs from the layer's explicit CRS; implicit transform is forbidden.
    CrsMismatch,
}

/// Tests a feature against one predicate without changing its semantic state.
///
/// - `feature`: Candidate feature record.
/// - `predicate`: Exact attribute or envelope predicate.
/// - `layer_crs`: CRS declared by the owning layer for spatial predicates.
///
/// # Returns
/// Whether the feature satisfies the predicate.
///
/// # Errors
/// Returns an error when an envelope query is malformed or uses a different CRS.
pub(crate) fn matches(
    feature: &Feature,
    predicate: &GeoPredicate,
    layer_crs: Option<&CoordinateReferenceSystem>,
) -> Result<bool, GeoQueryError> {
    match predicate {
        GeoPredicate::AttributeEquals { field, value } => {
            Ok(feature.attributes.get(field) == Some(value))
        }
        GeoPredicate::EnvelopeOverlaps { bounds } => {
            validate_bounds(bounds)?;
            if layer_crs != Some(&bounds.crs) {
                return Err(GeoQueryError::CrsMismatch);
            }
            let Some(geometry) = &feature.geometry else {
                return Ok(false);
            };
            let Some(envelope) = geometry_envelope(geometry)? else {
                return Ok(false);
            };
            Ok(envelope.0 <= Decimal::parse(&bounds.max_x)?
                && envelope.2 >= Decimal::parse(&bounds.min_x)?
                && envelope.1 <= Decimal::parse(&bounds.max_y)?
                && envelope.3 >= Decimal::parse(&bounds.min_y)?)
        }
    }
}

/// Validates query-level requirements even when a collection has no features.
///
/// - `predicate`: Query predicate to validate.
/// - `layer_crs`: CRS declared by the queried layer.
///
/// # Returns
/// Unit when bounds and their CRS context are valid.
///
/// # Errors
/// Returns an error for malformed bounds or a mismatched/unknown layer CRS.
pub(crate) fn validate_predicate(
    predicate: &GeoPredicate,
    layer_crs: Option<&CoordinateReferenceSystem>,
) -> Result<(), GeoQueryError> {
    if let GeoPredicate::EnvelopeOverlaps { bounds } = predicate {
        validate_bounds(bounds)?;
        if layer_crs != Some(&bounds.crs) {
            return Err(GeoQueryError::CrsMismatch);
        }
    }
    Ok(())
}

/// Validates bound ordering and numeric representation.
///
/// - `bounds`: Bounds to validate.
///
/// # Returns
/// Unit after all four coordinates are finite and min/max ordering is valid.
///
/// # Errors
/// Returns `InvalidBounds` for malformed or reversed coordinates.
pub(crate) fn validate_bounds(bounds: &GeoBounds) -> Result<(), GeoQueryError> {
    let min_x = Decimal::parse(&bounds.min_x)?;
    let min_y = Decimal::parse(&bounds.min_y)?;
    let max_x = Decimal::parse(&bounds.max_x)?;
    let max_y = Decimal::parse(&bounds.max_y)?;
    if min_x <= max_x && min_y <= max_y {
        Ok(())
    } else {
        Err(GeoQueryError::InvalidBounds)
    }
}

/// Computes the XY envelope of any supported geometry recursively.
///
/// - `geometry`: Semantic geometry to inspect.
///
/// # Returns
/// Minimum x, minimum y, maximum x, and maximum y, or `None` for empty geometry.
///
/// # Errors
/// Returns `InvalidBounds` when a coordinate is not valid plain decimal text.
pub(crate) fn geometry_envelope(
    geometry: &Geometry,
) -> Result<Option<(Decimal, Decimal, Decimal, Decimal)>, GeoQueryError> {
    let mut envelope: Option<(Decimal, Decimal, Decimal, Decimal)> = None;
    extend_envelope(geometry, &mut envelope)?;
    Ok(envelope)
}

/// Stores a normalized arbitrary-width decimal used for exact envelope comparisons.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Decimal {
    /// Whether this value is less than zero.
    negative: bool,
    /// Normalized integer digits without leading zeros.
    integer: String,
    /// Normalized fractional digits without trailing zeros.
    fraction: String,
}

impl Decimal {
    /// Parses signed plain decimal notation without rounding.
    ///
    /// - `value`: Decimal text without exponent notation.
    ///
    /// # Returns
    /// Normalized exact decimal value.
    ///
    /// # Errors
    /// Returns `InvalidBounds` for empty, malformed, or exponent-form input.
    pub(crate) fn parse(value: &str) -> Result<Self, GeoQueryError> {
        let (negative, unsigned) = match value.strip_prefix('-') {
            Some(unsigned) => (true, unsigned),
            None => (false, value.strip_prefix('+').unwrap_or(value)),
        };
        let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if integer.is_empty()
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(GeoQueryError::InvalidBounds);
        }
        let integer = integer.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        let is_zero = integer.is_empty() && fraction.is_empty();
        Ok(Self {
            negative: negative && !is_zero,
            integer: if integer.is_empty() {
                "0".to_owned()
            } else {
                integer.to_owned()
            },
            fraction: fraction.to_owned(),
        })
    }

    /// Compares absolute decimal magnitudes without machine-width conversion.
    ///
    /// - `other`: Other normalized decimal.
    ///
    /// # Returns
    /// Ordering by absolute value.
    fn abs_cmp(&self, other: &Self) -> Ordering {
        self.integer
            .len()
            .cmp(&other.integer.len())
            .then_with(|| self.integer.cmp(&other.integer))
            .then_with(|| {
                let width = self.fraction.len().max(other.fraction.len());
                for index in 0..width {
                    let left = self.fraction.as_bytes().get(index).copied().unwrap_or(b'0');
                    let right = other
                        .fraction
                        .as_bytes()
                        .get(index)
                        .copied()
                        .unwrap_or(b'0');
                    match left.cmp(&right) {
                        Ordering::Equal => {}
                        ordering => return ordering,
                    }
                }
                Ordering::Equal
            })
    }
}

impl Ord for Decimal {
    /// Orders normalized decimals by their exact signed numeric value.
    ///
    /// - `other`: Other normalized decimal.
    ///
    /// # Returns
    /// Numeric ordering.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => self.abs_cmp(other),
            (true, true) => other.abs_cmp(self),
        }
    }
}

impl PartialOrd for Decimal {
    /// Provides a total ordering consistent with exact decimal equality.
    ///
    /// - `other`: Other normalized decimal.
    ///
    /// # Returns
    /// Always returns the exact numeric ordering.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Updates an envelope while traversing nested geometry without a coordinate buffer.
///
/// - `geometry`: Geometry whose coordinates are traversed.
/// - `envelope`: Exact decimal accumulator updated in place.
///
/// # Returns
/// Unit after all coordinates have been included.
///
/// # Errors
/// Returns `InvalidBounds` when a coordinate is malformed decimal text.
fn extend_envelope(
    geometry: &Geometry,
    envelope: &mut Option<(Decimal, Decimal, Decimal, Decimal)>,
) -> Result<(), GeoQueryError> {
    match geometry {
        Geometry::Point(point) => extend_point(point, envelope)?,
        Geometry::LineString(points) => {
            for point in points {
                extend_point(point, envelope)?;
            }
        }
        Geometry::Polygon(rings) => {
            for point in rings.iter().flatten() {
                extend_point(point, envelope)?;
            }
        }
        Geometry::Collection(items) => {
            for item in items {
                extend_envelope(item, envelope)?;
            }
        }
    }
    Ok(())
}

/// Includes one coordinate in an exact decimal envelope.
///
/// - `point`: Coordinate whose x and y ordinates are parsed.
/// - `envelope`: Exact decimal accumulator updated in place.
///
/// # Returns
/// Unit after the coordinate has been included.
///
/// # Errors
/// Returns `InvalidBounds` when an ordinate is malformed decimal text.
fn extend_point(
    point: &Coordinate,
    envelope: &mut Option<(Decimal, Decimal, Decimal, Decimal)>,
) -> Result<(), GeoQueryError> {
    let x = Decimal::parse(&point.x)?;
    let y = Decimal::parse(&point.y)?;
    *envelope = Some(match envelope.take() {
        Some((min_x, min_y, max_x, max_y)) => (
            min_x.min(x.clone()),
            min_y.min(y.clone()),
            max_x.max(x.clone()),
            max_y.max(y.clone()),
        ),
        None => (x.clone(), y.clone(), x, y),
    });
    Ok(())
}

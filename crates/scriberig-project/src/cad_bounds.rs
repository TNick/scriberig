//! Queries approximate CAD geometry bounds without changing exact coordinates.

use std::cell::Cell;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_cad::{CadEntity, Coordinate, DrawingEntity, DrawingSpace};

use crate::{ProjectError, ProjectService};

/// Defines a two-dimensional CAD selection envelope in drawing units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadBounds2D {
    /// Minimum x ordinate in decimal text.
    pub min_x: String,
    /// Minimum y ordinate in decimal text.
    pub min_y: String,
    /// Maximum x ordinate in decimal text.
    pub max_x: String,
    /// Maximum y ordinate in decimal text.
    pub max_y: String,
}

/// Describes one exact-snapshot spatial query over a CAD drawing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadBoundsQueryRequest {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot to inspect.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// CAD coordinate space to query.
    pub space: DrawingSpace,
    /// Requested minimum and maximum x/y values.
    pub bounds: CadBounds2D,
    /// Zero-based result offset.
    pub offset: usize,
    /// Page size from one through five hundred.
    pub limit: usize,
}

/// Contains a bounded page of entities intersecting a CAD selection envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadBoundsEntityPage {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact snapshot used for the query.
    pub snapshot: SnapshotId,
    /// Semantic drawing identity.
    pub drawing: EntityId,
    /// CAD space searched.
    pub space: DrawingSpace,
    /// Query envelope in exact decimal text.
    pub bounds: CadBounds2D,
    /// Matching semantic entities.
    pub entities: Vec<DrawingEntity>,
    /// Total matching entities before pagination.
    pub total: usize,
    /// Whether more results remain.
    pub has_more: bool,
    /// Count of entities with invalid or unsupported extents.
    pub unbounded_entities: usize,
    /// Whether every entity in the requested space had a calculable extent.
    /// A complete extent does not mean the bounds reproduce rendered glyphs.
    pub complete: bool,
}

/// Represents a finite axis-aligned two-dimensional envelope.
#[derive(Clone, Copy, Debug)]
struct NumericBounds {
    /// Minimum x ordinate.
    min_x: f64,
    /// Minimum y ordinate.
    min_y: f64,
    /// Maximum x ordinate.
    max_x: f64,
    /// Maximum y ordinate.
    max_y: f64,
}

impl ProjectService {
    /// Finds bounded entities intersecting an envelope in one CAD space.
    ///
    /// Comparisons use finite floating-point values while preserving exact
    /// source coordinates. Entity extents expand conservatively to reduce
    /// boundary misses. Arcs use full-circle bounds; text and block references
    /// use insertion points. Results expose incomplete extent counts, and a
    /// complete result means numeric extents were available, not exact outlines.
    ///
    /// # Parameters
    /// # Parameters
    /// - `self`: Project service owning the project and snapshot stores.
    /// - `request`: Exact-snapshot query parameters and bounds.
    ///
    /// # Returns
    /// Bounded matching entities and extent completeness metadata.
    ///
    /// # Errors
    /// Returns invalid bounds/page, stale snapshot, drawing, or decode errors.
    pub fn cad_entities_intersecting_bounds(
        &self,
        request: CadBoundsQueryRequest,
    ) -> Result<CadBoundsEntityPage, ProjectError> {
        let query = NumericBounds::parse(&request.bounds)?;
        let drawing_view = self.cad_drawing(request.project, request.snapshot, request.drawing)?;
        let unbounded = Cell::new(0usize);
        let page = self.query_cad_entities(
            request.project,
            request.snapshot,
            request.drawing,
            request.offset,
            request.limit,
            |entity| {
                if entity.space != request.space {
                    return false;
                }
                if let Some(extent) = entity_bounds(entity) {
                    query.intersects(extent)
                } else {
                    unbounded.set(unbounded.get().saturating_add(1));
                    false
                }
            },
        )?;
        let unbounded_entities = unbounded.get();
        Ok(CadBoundsEntityPage {
            project: request.project,
            snapshot: request.snapshot,
            drawing: request.drawing,
            space: request.space,
            bounds: request.bounds,
            entities: page.entities,
            total: page.total,
            has_more: page.has_more,
            unbounded_entities,
            complete: unbounded_entities == 0 && drawing_view.complete,
        })
    }
}

impl NumericBounds {
    /// Parses finite ordered selection bounds.
    ///
    /// # Parameters
    /// # Parameters
    /// - `bounds`: Minimum and maximum decimal ordinates.
    ///
    /// # Returns
    /// Finite numeric envelope for approximate comparisons.
    ///
    /// # Errors
    /// Returns an error for malformed, non-finite, or reversed bounds.
    fn parse(bounds: &CadBounds2D) -> Result<Self, ProjectError> {
        let parsed = Self {
            min_x: parse_bound(&bounds.min_x)?,
            min_y: parse_bound(&bounds.min_y)?,
            max_x: parse_bound(&bounds.max_x)?,
            max_y: parse_bound(&bounds.max_y)?,
        };
        if parsed.min_x > parsed.max_x || parsed.min_y > parsed.max_y {
            return Err(bounds_error(
                "cad_bounds_invalid",
                "CAD selection bounds are reversed",
            ));
        }
        Ok(parsed)
    }

    /// Checks inclusive intersection with another envelope.
    ///
    /// # Parameters
    /// # Parameters
    /// - `self`: Query envelope.
    /// - `other`: Entity envelope.
    ///
    /// # Returns
    /// True when both x and y intervals overlap.
    fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

/// Derives an approximate envelope for one CAD geometry value.
///
/// # Parameters
/// # Parameters
/// - `entity`: Semantic entity to bound.
///
/// # Returns
/// Extent, or None when required coordinate text is invalid.
fn entity_bounds(entity: &DrawingEntity) -> Option<NumericBounds> {
    match &entity.geometry {
        CadEntity::Line { start, end } => points_bounds([start, end]),
        CadEntity::Polyline { vertices, .. } => points_bounds(vertices.iter()),
        CadEntity::Circle { center, radius } | CadEntity::Arc { center, radius, .. } => {
            let (x, y) = point_xy(center)?;
            let radius = radius.parse::<f64>().ok()?;
            if !radius.is_finite() || radius < 0.0 {
                return None;
            }
            expanded_bounds(x - radius, y - radius, x + radius, y + radius)
        }
        CadEntity::Text { position, .. } => point_bounds(position),
        CadEntity::Dimension {
            definition_points, ..
        } => points_bounds(definition_points.iter()),
        CadEntity::BlockReference(reference) => point_bounds(&reference.insertion),
    }
}

/// Builds an expanded envelope around one coordinate.
///
/// # Parameters
/// # Parameters
/// - `coordinate`: Exact CAD coordinate.
///
/// # Returns
/// Point extent or None when ordinates are invalid.
fn point_bounds(coordinate: &Coordinate) -> Option<NumericBounds> {
    let (x, y) = point_xy(coordinate)?;
    expanded_bounds(x, y, x, y)
}

/// Builds an envelope around an ordered coordinate collection.
///
/// # Parameters
/// - coordinates: Coordinates defining the entity.
///
/// # Returns
/// Expanded extent or None when the collection is empty or malformed.
fn points_bounds<'a>(
    coordinates: impl IntoIterator<Item = &'a Coordinate>,
) -> Option<NumericBounds> {
    let mut points = coordinates.into_iter().map(point_xy);
    let (x, y) = points.next()??;
    let mut bounds = NumericBounds {
        min_x: x,
        min_y: y,
        max_x: x,
        max_y: y,
    };
    for point in points {
        let (x, y) = point?;
        bounds.min_x = bounds.min_x.min(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_x = bounds.max_x.max(x);
        bounds.max_y = bounds.max_y.max(y);
    }
    expanded_bounds(bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y)
}

/// Expands an envelope outward and rejects overflowed coordinates.
///
/// # Parameters
/// - `min_x`: Unexpanded minimum x.
/// - `min_y`: Unexpanded minimum y.
/// - `max_x`: Unexpanded maximum x.
/// - `max_y`: Unexpanded maximum y.
///
/// # Returns
/// Finite envelope expanded conservatively, or None on overflow.
fn expanded_bounds(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Option<NumericBounds> {
    let margin = |value: f64| value.abs().max(1.0) * f64::EPSILON * 4.0;
    let bounds = NumericBounds {
        min_x: min_x - margin(min_x),
        min_y: min_y - margin(min_y),
        max_x: max_x + margin(max_x),
        max_y: max_y + margin(max_y),
    };
    [bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y]
        .into_iter()
        .all(f64::is_finite)
        .then_some(bounds)
}

/// Parses finite x and y ordinates for a two-dimensional comparison.
///
/// # Parameters
/// # Parameters
/// - `coordinate`: Exact-text coordinate.
///
/// # Returns
/// Finite approximate x/y pair.
fn point_xy(coordinate: &Coordinate) -> Option<(f64, f64)> {
    Some((
        coordinate
            .x
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())?,
        coordinate
            .y
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())?,
    ))
}

/// Parses one finite selection-envelope ordinate.
///
/// # Parameters
/// # Parameters
/// - `value`: Decimal text to parse.
///
/// # Returns
/// Finite floating-point ordinate.
///
/// # Errors
/// Returns an error when the value is invalid or non-finite.
fn parse_bound(value: &str) -> Result<f64, ProjectError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| {
            bounds_error(
                "cad_bounds_invalid",
                "CAD bounds require finite numeric ordinates",
            )
        })
}

/// Creates a structured CAD bounds error.
///
/// # Parameters
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project error without source package bytes.
fn bounds_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}

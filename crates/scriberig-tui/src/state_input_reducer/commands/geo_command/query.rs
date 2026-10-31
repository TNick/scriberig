//! Parses typed bounded geospatial query command arguments.

use super::scope;
use crate::state::TuiState;

/// Builds a bounded exact-snapshot predicate query from terminal arguments.
///
/// Syntax is `/geo-query <layer-id> <predicate-json> [offset]`, where the JSON
/// value uses the stable `GeoFeaturePredicate` wire shape.
///
/// # Parameters
/// - `state`: TUI state with an active project, snapshot, and loaded catalog.
/// - `arguments`: Text following `/geo-query`.
///
/// # Returns
/// Exact application query request with a forty-feature page bound.
///
/// # Errors
/// Returns a usage message for missing scope, malformed JSON, or invalid offset.
pub fn query_request(
    state: &TuiState,
    arguments: &str,
) -> Result<scriberig_app::GeoFeatureQuery, String> {
    let (project, snapshot) = scope(state).map_err(str::to_owned)?;
    let path = state
        .geo_path
        .clone()
        .ok_or_else(|| "Load a GeoPackage catalog before querying a layer.".to_owned())?;
    let mut parts = arguments.splitn(2, char::is_whitespace);
    let layer = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Usage: /geo-query <layer-id> <predicate-json> [offset].".to_owned())?
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| "Layer ID must be a semantic entity ID.".to_owned())?;
    let rest = parts.next().unwrap_or_default().trim();
    if rest.is_empty() {
        return Err("Usage: /geo-query <layer-id> <predicate-json> [offset].".to_owned());
    }
    let (predicate_json, offset) = match rest.rsplit_once(char::is_whitespace) {
        Some((json, tail)) if tail.trim().parse::<usize>().is_ok() => {
            (json.trim(), tail.trim().parse().expect("parsed offset"))
        }
        _ => (rest, 0),
    };
    let predicate = serde_json::from_str::<scriberig_protocol::GeoFeaturePredicate>(predicate_json)
        .map_err(|_| "Predicate must be valid GeoFeaturePredicate JSON.".to_owned())?;
    let predicate = match predicate {
        scriberig_protocol::GeoFeaturePredicate::AttributeEquals { field, value } => {
            scriberig_domain_geo::GeoPredicate::AttributeEquals {
                field,
                value: crate::view::geo::value_from_protocol(value),
            }
        }
        scriberig_protocol::GeoFeaturePredicate::EnvelopeOverlaps { bounds } => {
            scriberig_domain_geo::GeoPredicate::EnvelopeOverlaps {
                bounds: scriberig_domain_geo::GeoBounds {
                    min_x: bounds.min_x,
                    min_y: bounds.min_y,
                    max_x: bounds.max_x,
                    max_y: bounds.max_y,
                    crs: scriberig_domain_geo::CoordinateReferenceSystem {
                        authority: bounds.crs.authority,
                        code: bounds.crs.code,
                        definition: bounds.crs.definition,
                    },
                },
            }
        }
    };
    Ok(scriberig_app::GeoFeatureQuery {
        project,
        snapshot,
        path,
        layer,
        predicate,
        offset,
        limit: 40,
    })
}

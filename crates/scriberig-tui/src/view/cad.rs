//! Normalizes and formats exact-snapshot CAD views for the terminal.

use std::fmt::Write as _;

use crate::inspector::LoadState;
use scriberig_protocol::{
    CadCoordinate, CadDrawingInspectResponse, CadDrawingSpace, CadEntitiesResponse,
    CadEntityStyleRef, CadEntitySummary, CadGeometry, CadLayerChangeResponse, CadLayerSummary,
};

/// Converts one authorized project drawing summary to its protocol view.
///
/// # Parameters
/// - `drawing`: Exact-snapshot project drawing summary.
///
/// # Returns
/// Parser-independent drawing and layer metadata.
#[must_use]
pub fn drawing_from_application(
    drawing: scriberig_project::CadDrawingView,
) -> CadDrawingInspectResponse {
    CadDrawingInspectResponse {
        project: drawing.project.to_string(),
        snapshot: drawing.snapshot.to_string(),
        drawing: drawing.drawing.to_string(),
        units: drawing.units,
        coordinate_reference: drawing.coordinate_reference,
        layers: drawing
            .layers
            .into_iter()
            .map(|layer| CadLayerSummary {
                id: layer.id.to_string(),
                name: layer.name,
                visible: layer.visible,
                locked: layer.locked,
            })
            .collect(),
        entity_count: drawing.entity_count,
        complete: drawing.complete,
        diagnostics: drawing.diagnostics,
    }
}

/// Converts one authorized project entity page to its protocol view.
///
/// # Parameters
/// - `page`: Exact-snapshot project entity page.
///
/// # Returns
/// Bounded parser-independent entity summaries.
#[must_use]
pub fn entities_from_application(page: scriberig_project::CadEntityPage) -> CadEntitiesResponse {
    CadEntitiesResponse {
        project: page.project.to_string(),
        snapshot: page.snapshot.to_string(),
        drawing: page.drawing.to_string(),
        entities: page
            .entities
            .into_iter()
            .map(|entity| CadEntitySummary {
                id: entity.id.to_string(),
                layer: entity.layer.to_string(),
                space: match entity.space {
                    scriberig_domain_cad::DrawingSpace::Model => CadDrawingSpace::Model,
                    scriberig_domain_cad::DrawingSpace::Paper(id) => {
                        CadDrawingSpace::Paper(id.to_string())
                    }
                    scriberig_domain_cad::DrawingSpace::Named(name) => CadDrawingSpace::Named(name),
                },
                geometry: geometry_from_application(entity.geometry),
                style: entity.style.map(|style| CadEntityStyleRef {
                    key: style.key,
                    adapter: style.adapter,
                }),
            })
            .collect(),
        total: page.total,
        has_more: page.has_more,
    }
}

/// Converts one domain geometry to its stable protocol variant.
///
/// # Parameters
/// - `geometry`: Typed format-independent CAD geometry.
///
/// # Returns
/// Equivalent protocol geometry with exact decimal values retained.
fn geometry_from_application(geometry: scriberig_domain_cad::CadEntity) -> CadGeometry {
    use scriberig_domain_cad::CadEntity as Domain;
    match geometry {
        Domain::Line { start, end } => CadGeometry::Line {
            start: coordinate_from_application(start),
            end: coordinate_from_application(end),
        },
        Domain::Polyline { vertices, closed } => CadGeometry::Polyline {
            vertices: vertices
                .into_iter()
                .map(coordinate_from_application)
                .collect(),
            closed,
        },
        Domain::Circle { center, radius } => CadGeometry::Circle {
            center: coordinate_from_application(center),
            radius,
        },
        Domain::Arc {
            center,
            radius,
            start_angle,
            end_angle,
        } => CadGeometry::Arc {
            center: coordinate_from_application(center),
            radius,
            start_angle,
            end_angle,
        },
        Domain::Text {
            position,
            value,
            height,
        } => CadGeometry::Text {
            position: coordinate_from_application(position),
            value,
            height,
        },
        Domain::Dimension {
            definition_points,
            text,
        } => CadGeometry::Dimension {
            definition_points: definition_points
                .into_iter()
                .map(coordinate_from_application)
                .collect(),
            text,
        },
        Domain::BlockReference(reference) => CadGeometry::BlockReference {
            definition: reference.definition.to_string(),
            insertion: coordinate_from_application(reference.insertion),
            scale: reference.scale,
            rotation: reference.rotation,
        },
    }
}

/// Converts one exact domain coordinate to its wire representation.
///
/// # Parameters
/// - `coordinate`: Exact decimal coordinate in drawing units.
///
/// # Returns
/// Protocol coordinate with authored decimal spellings preserved.
fn coordinate_from_application(coordinate: scriberig_domain_cad::Coordinate) -> CadCoordinate {
    CadCoordinate {
        x: coordinate.x,
        y: coordinate.y,
        z: coordinate.z,
    }
}

/// Converts a CAD layer candidate into the shared review response shape.
///
/// # Parameters
/// - `candidate`: Durable project candidate returned by the application.
///
/// # Returns
/// Protocol candidate metadata for the common TUI review flow.
#[must_use]
pub fn layer_candidate_from_application(
    candidate: &scriberig_app::CadLayerCandidate,
) -> CadLayerChangeResponse {
    let validation = match &candidate.validation {
        scriberig_project::CandidateValidation::Pending => {
            scriberig_protocol::CadCandidateValidation::Pending
        }
        scriberig_project::CandidateValidation::Passed => {
            scriberig_protocol::CadCandidateValidation::Passed
        }
        scriberig_project::CandidateValidation::Failed { diagnostics } => {
            scriberig_protocol::CadCandidateValidation::Failed {
                diagnostics: diagnostics.clone(),
            }
        }
    };
    CadLayerChangeResponse {
        candidate: candidate.id.clone(),
        project: candidate.project.to_string(),
        base_snapshot: candidate.base_snapshot.to_string(),
        path: candidate.path.clone(),
        validation,
    }
}

/// Converts a project candidate into the shared CAD protocol response.
///
/// # Parameters
/// - `candidate`: Durable candidate returned by the authorized application operation.
///
/// # Returns
/// Typed candidate metadata for the remote and embedded TUI paths.
#[must_use]
pub fn layer_candidate_from_project(
    candidate: &scriberig_app::CadLayerCandidate,
) -> CadLayerChangeResponse {
    layer_candidate_from_application(candidate)
}

/// Renders bounded CAD layers, typed entity properties, and candidate status.
///
/// # Parameters
/// - `drawing`: Exact-snapshot drawing load state.
/// - `entities`: Bounded exact-snapshot entity page state.
/// - `candidate`: Latest layer-change candidate state.
/// - `offset`: Zero-based source-order offset for the visible entity page.
///
/// # Returns
/// Markdown suitable for the upstream RPI semantic surface.
#[must_use]
pub fn render(
    drawing: &LoadState<CadDrawingInspectResponse>,
    entities: &LoadState<CadEntitiesResponse>,
    candidate: &LoadState<CadLayerChangeResponse>,
    offset: usize,
) -> String {
    let mut output = String::from("# CAD drawing\n\n");
    match drawing {
        LoadState::Ready(view) => {
            let _ = write!(
                output,
                "Drawing: `{}`\n\nUnits: {}\n\n",
                view.drawing, view.units
            );
            let _ = write!(
                output,
                "Fidelity: {}\n\n",
                if view.complete {
                    "complete"
                } else {
                    "incomplete; edits are restricted"
                }
            );
            output.push_str("## Layers\n\n");
            for layer in &view.layers {
                let _ = writeln!(
                    output,
                    "- `{}` **{}**{}{}\n",
                    layer.id,
                    layer.name,
                    if layer.visible { "" } else { " (hidden)" },
                    if layer.locked { " (locked)" } else { "" },
                );
            }
            for diagnostic in &view.diagnostics {
                let _ = writeln!(output, "\nDiagnostic: {diagnostic}");
            }
        }
        other => append_load_state(&mut output, other, "Drawing summary"),
    }
    output.push_str("\n## Entities\n\n");
    match entities {
        LoadState::Ready(page) => {
            let _ = writeln!(
                output,
                "Entities {}–{} of {}\n\n",
                offset + usize::from(!page.entities.is_empty()),
                offset + page.entities.len(),
                page.total
            );
            for entity in &page.entities {
                let _ = writeln!(
                    output,
                    "- `{}` · layer `{}` · {}\n",
                    entity.id,
                    entity.layer,
                    geometry_summary(&entity.geometry),
                );
            }
            if page.has_more {
                output.push_str(
                    "\nMore entities are available. Use `/cad <next-offset>` to continue.\n",
                );
            }
        }
        other => append_load_state(&mut output, other, "Entity page"),
    }
    match candidate {
        LoadState::Ready(value) => {
            let _ = write!(
                output,
                "\n## Layer change candidate\n\nCandidate: `{}`\n\nSource: `{}`\n\nValidation: {:?}\n",
                value.candidate, value.path, value.validation
            );
        }
        LoadState::Failed { message, .. } => {
            let _ = writeln!(output, "\nLayer change failed: {message}");
        }
        _ => {}
    }
    output
}

/// Appends a concise loading or failure status to the CAD view.
///
/// # Parameters
/// - `output`: Markdown buffer receiving the status.
/// - `state`: State to describe.
/// - `label`: Human-readable model name.
fn append_load_state<T>(output: &mut String, state: &LoadState<T>, label: &str) {
    match state {
        LoadState::NotRequested => {
            let _ = writeln!(output, "{label} not loaded.");
        }
        LoadState::Loading { .. } => {
            let _ = writeln!(output, "Loading {label}…");
        }
        LoadState::Ready(_) => {}
        LoadState::Failed { message, .. } => {
            let _ = writeln!(output, "{label} failed: {message}");
        }
    }
}

/// Summarizes entity geometry without dumping unbounded vertex arrays.
///
/// # Parameters
/// - `geometry`: Typed CAD geometry from the semantic protocol.
///
/// # Returns
/// Entity kind and salient authored values.
fn geometry_summary(geometry: &CadGeometry) -> String {
    match geometry {
        CadGeometry::Line { start, end } => format!("Line {start:?} → {end:?}"),
        CadGeometry::Polyline { vertices, closed } => {
            format!("Polyline ({} vertices, closed: {closed})", vertices.len())
        }
        CadGeometry::Circle { center, radius } => {
            format!("Circle center {center:?}, radius {radius}")
        }
        CadGeometry::Arc {
            center,
            radius,
            start_angle,
            end_angle,
        } => format!("Arc center {center:?}, radius {radius}, angles {start_angle}–{end_angle}"),
        CadGeometry::Text {
            position,
            value,
            height,
        } => {
            format!("Text {value:?} at {position:?}, height {height}")
        }
        CadGeometry::Dimension {
            definition_points,
            text,
        } => format!(
            "Dimension ({} points, text: {})",
            definition_points.len(),
            text.as_deref().unwrap_or("none")
        ),
        CadGeometry::BlockReference {
            definition,
            insertion,
            scale,
            rotation,
        } => format!(
            "Block reference `{definition}` at {insertion:?}, scale {scale:?}, rotation {rotation}"
        ),
    }
}

#[cfg(test)]
#[path = "cad_tests.rs"]
mod tests;

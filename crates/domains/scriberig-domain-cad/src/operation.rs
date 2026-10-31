//! Applies typed structure changes to in-memory CAD drawing semantics.

use scriberig_core::EntityId;

use crate::Drawing;

/// Identifies a rejected semantic CAD operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadOperationError {
    /// Stable operation failure category.
    pub code: &'static str,
}

/// Moves one semantic entity to an existing layer in the same drawing.
///
/// The entity identity and ordering remain stable; only its explicit layer
/// membership changes. Callers persist the updated entity as a new immutable
/// revision through the shared change model.
///
/// # Parameters
/// - `drawing`: Drawing that owns the entity and destination layer.
/// - `entity`: Stable semantic entity identity to move.
/// - `destination_layer`: Existing semantic layer identity in `drawing`.
///
/// # Returns
/// Unit after the entity's layer relation is updated.
///
/// # Errors
/// Returns `cad_entity_not_found` or `cad_layer_not_found` when either identity
/// is not present in the supplied drawing.
pub fn move_entity_to_layer(
    drawing: &mut Drawing,
    entity: EntityId,
    destination_layer: EntityId,
) -> Result<(), CadOperationError> {
    if !drawing
        .layers
        .iter()
        .any(|layer| layer.id == destination_layer)
    {
        return Err(CadOperationError {
            code: "cad_layer_not_found",
        });
    }
    let Some(target) = drawing
        .entities
        .iter_mut()
        .find(|candidate| candidate.id == entity)
    else {
        return Err(CadOperationError {
            code: "cad_entity_not_found",
        });
    };
    target.layer = destination_layer;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Verifies entity layer membership changes without changing other semantics.

    use scriberig_core::EntityId;

    use crate::{CadEntity, Coordinate, Drawing, DrawingEntity, DrawingSpace, Layer};

    use super::move_entity_to_layer;

    /// Moves one line to an existing layer while preserving neighboring entity data.
    #[test]
    fn moving_entity_updates_only_its_layer_relation() {
        let source_layer = Layer {
            id: EntityId::new(),
            name: "SOURCE".to_owned(),
            visible: true,
            locked: false,
        };
        let destination_layer = Layer {
            id: EntityId::new(),
            name: "DESTINATION".to_owned(),
            visible: true,
            locked: false,
        };
        let target = DrawingEntity {
            id: EntityId::new(),
            layer: source_layer.id,
            space: DrawingSpace::Model,
            geometry: CadEntity::Line {
                start: Coordinate::new("0", "0", "0"),
                end: Coordinate::new("1", "0", "0"),
            },
            style: None,
        };
        let neighbor = DrawingEntity {
            id: EntityId::new(),
            layer: source_layer.id,
            space: DrawingSpace::Model,
            geometry: CadEntity::Text {
                position: Coordinate::new("2", "0", "0"),
                value: "keep".to_owned(),
                height: "1".to_owned(),
            },
            style: None,
        };
        let mut drawing = Drawing {
            id: EntityId::new(),
            units: "millimeter".to_owned(),
            coordinate_reference: None,
            layers: vec![source_layer.clone(), destination_layer.clone()],
            entities: vec![target.clone(), neighbor.clone()],
            blocks: Vec::new(),
            layouts: Vec::new(),
            external_references: Vec::new(),
        };

        move_entity_to_layer(&mut drawing, target.id, destination_layer.id)
            .expect("destination layer exists");

        assert_eq!(drawing.entities[0].layer, destination_layer.id);
        assert_eq!(drawing.entities[0].geometry, target.geometry);
        assert_eq!(drawing.entities[1], neighbor);
    }

    /// Rejects layer moves that name no layer in the drawing.
    #[test]
    fn moving_entity_to_missing_layer_fails_without_mutation() {
        let layer = Layer {
            id: EntityId::new(),
            name: "SOURCE".to_owned(),
            visible: true,
            locked: false,
        };
        let entity = DrawingEntity {
            id: EntityId::new(),
            layer: layer.id,
            space: DrawingSpace::Model,
            geometry: CadEntity::Line {
                start: Coordinate::new("0", "0", "0"),
                end: Coordinate::new("1", "0", "0"),
            },
            style: None,
        };
        let mut drawing = Drawing {
            id: EntityId::new(),
            units: "millimeter".to_owned(),
            coordinate_reference: None,
            layers: vec![layer],
            entities: vec![entity.clone()],
            blocks: Vec::new(),
            layouts: Vec::new(),
            external_references: Vec::new(),
        };

        let error = move_entity_to_layer(&mut drawing, entity.id, EntityId::new())
            .expect_err("unknown destination layer is rejected");

        assert_eq!(error.code, "cad_layer_not_found");
        assert_eq!(drawing.entities[0], entity);
    }
}

//! Imports supported DXF entities and exposes a bounded safe edit operation.

use std::collections::BTreeMap;
use std::io::Cursor;

use scriberig_core::EntityId;
use scriberig_domain_cad::{
    CadEntity, CadProviderEntityIdentifier, CadProviderIdentity, CadProviderImport, Coordinate,
    Drawing, DrawingEntity, DrawingSpace, Layer,
};

mod edit;
mod reconciliation;

use reconciliation::DxfEntityReconciler;

pub use edit::{
    DxfEditError, change_entity_layer, replace_line_geometry, replace_text,
    supports_opaque_preserving_edits, verify_opaque_preservation,
};

/// Reports the supported DXF semantic subset and its conservative capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DxfImport {
    /// Shared drawing semantics and external DXF identifiers.
    pub provider_import: CadProviderImport,
    /// Exact source package digest.
    pub source_digest: [u8; 32],
    /// Whether the complete drawing mapped to the supported semantic subset.
    pub complete: bool,
    /// Safe reasons that the imported drawing is read-only.
    pub diagnostics: Vec<String>,
}

impl DxfImport {
    /// Returns the format-independent semantic drawing.
    ///
    /// # Parameters
    /// - `self`: Imported DXF result.
    ///
    /// # Returns
    /// The CAD drawing without DXF-native object handles.
    #[must_use]
    pub fn drawing(&self) -> &Drawing {
        self.provider_import.drawing()
    }
}

/// Associates prior semantic identity with detached DXF source evidence.
///
/// Public members: `semantic` and `native_handle`.
/// Private members: none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DxfPreviousEntity {
    /// Prior provider-neutral CAD entity.
    pub semantic: DrawingEntity,
    /// Prior DXF handle held outside the semantic entity value.
    pub native_handle: Option<String>,
}

/// Imports a DXF package into the shared CAD semantic model.
///
/// - `bytes`: Exact source DXF bytes.
/// - `drawing_id`: Caller-owned stable semantic drawing identity.
///
/// Supported layers and entities with exact source digest evidence.
///
/// # Returns
/// Parsed semantic drawing and exact source digest evidence.
///
/// Returns a parser error when the source is not a valid DXF document.
///
/// # Errors
/// Returns an error when the source bytes cannot be parsed as a DXF document.
pub fn import_dxf(bytes: &[u8], drawing_id: EntityId) -> Result<DxfImport, String> {
    import_dxf_reconciled(bytes, drawing_id, &[])
}

/// Imports DXF semantics and reconciles entity identities against a prior drawing.
///
/// Native handles are used only as external matching evidence; unmatched semantic
/// entities receive deterministic source-order identities scoped to the drawing.
///
/// # Parameters
/// - `bytes`: Exact source DXF bytes.
/// - `drawing_id`: Stable semantic drawing identity.
/// - `previous_entities`: Prior semantic entities with detached DXF source evidence.
///
/// # Returns
/// Imported drawing with semantic IDs reconciled where evidence is unambiguous.
///
/// # Errors
/// Returns an error when the source bytes cannot be parsed as a DXF document.
pub fn import_dxf_reconciled(
    bytes: &[u8],
    drawing_id: EntityId,
    previous_entities: &[DxfPreviousEntity],
) -> Result<DxfImport, String> {
    let parsed = dxf::Drawing::load(&mut Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut layer_ids = parsed
        .layers()
        .map(|layer| {
            (
                layer.name.clone(),
                derive_id(drawing_id, b"layer", layer.name.as_bytes()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for entity in parsed.entities() {
        layer_ids
            .entry(entity.common.layer.clone())
            .or_insert_with(|| derive_id(drawing_id, b"layer", entity.common.layer.as_bytes()));
    }
    if layer_ids.is_empty() {
        layer_ids.insert("0".to_owned(), derive_id(drawing_id, b"layer", b"0"));
    }
    let layers = layer_ids
        .iter()
        .map(|(name, id)| Layer {
            id: *id,
            name: name.clone(),
            visible: true,
            locked: false,
        })
        .collect::<Vec<_>>();
    let (entities, entity_identifiers, mut diagnostics, complete) =
        import_entities(&parsed, drawing_id, &layer_ids, previous_entities);
    if !complete {
        diagnostics.push("DXF write capability is disabled for this import".to_owned());
    }
    let drawing = Drawing {
        id: drawing_id,
        units: drawing_units(parsed.header.default_drawing_units).to_owned(),
        coordinate_reference: None,
        layers,
        entities,
        blocks: Vec::new(),
        layouts: Vec::new(),
        external_references: Vec::new(),
    };
    let provider_import = CadProviderImport::new(
        CadProviderIdentity {
            id: "scriberig.dxf".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        drawing,
        entity_identifiers,
    )
    .map_err(|error| format!("DXF provider provenance is invalid: {:?}", error.kind))?;
    Ok(DxfImport {
        provider_import,
        source_digest: *blake3::hash(bytes).as_bytes(),
        complete,
        diagnostics,
    })
}

/// Imports the supported entity subset and reports unsupported source evidence.
///
/// # Parameters
/// - `drawing`: Parsed source drawing.
/// - `drawing_id`: Stable semantic drawing identity.
/// - `layer_ids`: Stable layer identities derived from the source.
/// - `previous_entities`: Prior semantic entities with detached source handles.
///
/// # Returns
/// Imported entities, fidelity diagnostics, and whether the subset is complete.
fn import_entities(
    drawing: &dxf::Drawing,
    drawing_id: EntityId,
    layer_ids: &BTreeMap<String, EntityId>,
    previous_entities: &[DxfPreviousEntity],
) -> (
    Vec<DrawingEntity>,
    Vec<CadProviderEntityIdentifier>,
    Vec<String>,
    bool,
) {
    let mut diagnostics = Vec::new();
    let mut complete = true;
    if !drawing.classes.is_empty() {
        complete = false;
        diagnostics.push("DXF class definitions are not represented semantically".to_owned());
    }
    if drawing.blocks().next().is_some() {
        complete = false;
        diagnostics.push("DXF block records are not represented semantically".to_owned());
    }
    if drawing.objects().next().is_some() {
        complete = false;
        diagnostics.push("DXF object records are not represented semantically".to_owned());
    }
    let mut entities = Vec::new();
    let mut entity_identifiers = Vec::new();
    let mut reconciler = DxfEntityReconciler::new(previous_entities);
    for (ordinal, entity) in drawing.entities().enumerate() {
        let Some(geometry) = semantic_geometry(&entity.specific) else {
            complete = false;
            diagnostics.push(format!(
                "unsupported DXF entity on layer {} was not imported",
                entity.common.layer
            ));
            continue;
        };
        let native_handle = format!("{:X}", entity.common.handle.0);
        let layer = layer_ids[&entity.common.layer];
        let id = match reconciler.resolve(&native_handle, layer, &geometry) {
            Ok(Some(id)) => id,
            Ok(None) => derive_id(drawing_id, b"entity-slot", &(ordinal as u64).to_be_bytes()),
            Err(()) => {
                complete = false;
                diagnostics.push(format!(
                    "ambiguous DXF identity reconciliation for native handle {native_handle}; assigned a new semantic entity"
                ));
                derive_id(drawing_id, b"entity-slot", &(ordinal as u64).to_be_bytes())
            }
        };
        entities.push(DrawingEntity {
            id,
            layer,
            space: DrawingSpace::Model,
            geometry,
            style: None,
        });
        entity_identifiers.push(CadProviderEntityIdentifier {
            entity: id,
            value: native_handle,
        });
    }
    (entities, entity_identifiers, diagnostics, complete)
}

/// Maps one supported parser entity into the shared CAD geometry model.
///
/// - `entity`: Parser-owned DXF geometry.
///
/// # Returns
/// Shared semantic geometry, or `None` when this entity is unsupported.
fn semantic_geometry(entity: &dxf::entities::EntityType) -> Option<CadEntity> {
    match entity {
        dxf::entities::EntityType::Line(line) => Some(CadEntity::Line {
            start: coordinate(&line.p1),
            end: coordinate(&line.p2),
        }),
        dxf::entities::EntityType::Polyline(polyline) => Some(CadEntity::Polyline {
            vertices: polyline
                .vertices()
                .map(|vertex| coordinate(&vertex.location))
                .collect(),
            closed: polyline.is_closed(),
        }),
        dxf::entities::EntityType::LwPolyline(polyline) => Some(CadEntity::Polyline {
            vertices: polyline
                .vertices
                .iter()
                .map(|vertex| Coordinate::new(vertex.x.to_string(), vertex.y.to_string(), "0"))
                .collect(),
            closed: polyline.is_closed(),
        }),
        dxf::entities::EntityType::Circle(circle) => Some(CadEntity::Circle {
            center: coordinate(&circle.center),
            radius: circle.radius.to_string(),
        }),
        dxf::entities::EntityType::Arc(arc) => Some(CadEntity::Arc {
            center: coordinate(&arc.center),
            radius: arc.radius.to_string(),
            start_angle: arc.start_angle.to_string(),
            end_angle: arc.end_angle.to_string(),
        }),
        dxf::entities::EntityType::Text(text) => Some(CadEntity::Text {
            position: coordinate(&text.location),
            value: text.value.clone(),
            height: text.text_height.to_string(),
        }),
        _ => None,
    }
}

/// Converts the DXF header's insertion units to a stable adapter-neutral label.
///
/// - `units`: Unit code read from the exact DXF header.
///
/// # Returns
/// A canonical unit name, or an explicit unitless label when DXF specifies none.
fn drawing_units(units: dxf::enums::Units) -> &'static str {
    use dxf::enums::Units;

    match units {
        Units::Unitless => "dxf:unitless",
        Units::Inches => "inches",
        Units::Feet => "feet",
        Units::Miles => "miles",
        Units::Millimeters => "millimeters",
        Units::Centimeters => "centimeters",
        Units::Meters => "meters",
        Units::Kilometers => "kilometers",
        Units::Microinches => "microinches",
        Units::Mils => "mils",
        Units::Yards => "yards",
        Units::Angstroms => "angstroms",
        Units::Nanometers => "nanometers",
        Units::Microns => "microns",
        Units::Decimeters => "decimeters",
        Units::Decameters => "decameters",
        Units::Hectometers => "hectometers",
        Units::Gigameters => "gigameters",
        Units::AstronomicalUnits => "astronomical_units",
        Units::LightYears => "light_years",
        Units::Parsecs => "parsecs",
        Units::USSurveyFeet => "us_survey_feet",
        Units::USSurveyInch => "us_survey_inches",
        Units::USSurveyYard => "us_survey_yards",
        Units::USSurveyMile => "us_survey_miles",
    }
}

/// Converts one parser-native point to exact textual semantic ordinates.
///
/// - `point`: DXF point to convert.
///
/// Adapter-neutral coordinate values.
///
/// # Returns
/// Exact textual ordinates for the supplied point.
fn coordinate(point: &dxf::Point) -> Coordinate {
    Coordinate::new(
        point.x.to_string(),
        point.y.to_string(),
        point.z.to_string(),
    )
}

/// Derives deterministic identities for imported DXF semantic objects.
///
/// - `parent`: Stable semantic parent identity.
/// - `namespace`: Adapter-owned identity namespace.
/// - `key`: Exact source key within that namespace.
///
/// Deterministic Scriberig entity identity.
///
/// # Returns
/// Stable Scriberig identity derived from the parent and exact source key.
fn derive_id(parent: EntityId, namespace: &[u8], key: &[u8]) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-dxf-semantic-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(namespace);
    hasher.update(key);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

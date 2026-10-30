//! Reassigns provider-created identities to project-owned CAD identities.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::EntityId;
use scriberig_domain_cad::{
    CadEntity, CadProviderEntityIdentifier, CadProviderError, CadProviderErrorKind,
    CadProviderImport, Drawing, DrawingSpace,
};
use scriberig_domain_dxf::DxfPreviousEntity;

/// Normalizes provider-created IDs before shared semantic persistence.
///
/// Provider-native handles preserve identity within a provider lineage. When
/// the provider changes, entities are reconciled by exact semantics and then
/// by a unique typed-structure match; ambiguous continuity fails closed.
///
/// # Parameters
/// - `source`: Stable Scriberig identity of the exact source artifact.
/// - `imported`: Provider-neutral drawing plus detached native identifiers.
/// - `previous`: Prior semantic entities and their detached native handles.
/// - `same_provider`: Whether prior and current provider identity/version match.
///
/// # Returns
/// A drawing with Scriberig-owned IDs and native identifiers remapped to those IDs.
///
/// # Errors
/// Returns `InvalidImportProvenance` when provider relationships are inconsistent
/// or one native handle identifies more than one semantic entity.
#[allow(clippy::too_many_lines)]
pub(super) fn provider_drawing(
    source: EntityId,
    imported: &CadProviderImport,
    previous: &[DxfPreviousEntity],
    same_provider: bool,
) -> Result<(Drawing, Vec<CadProviderEntityIdentifier>), CadProviderError> {
    let source_drawing = imported.drawing();
    let drawing_id = semantic_id(source, b"drawing", b"root");
    let layer_ids = named_ids(
        source,
        b"layer",
        source_drawing
            .layers
            .iter()
            .map(|item| (item.id, item.name.as_str())),
    );
    let block_ids = named_ids(
        source,
        b"block",
        source_drawing
            .blocks
            .iter()
            .map(|item| (item.id, item.name.as_str())),
    );
    let layout_ids = named_ids(
        source,
        b"layout",
        source_drawing
            .layouts
            .iter()
            .map(|item| (item.id, item.name.as_str())),
    );
    let layers = source_drawing
        .layers
        .iter()
        .map(|layer| (layer.id, layer_ids[&layer.id]))
        .collect::<BTreeMap<_, _>>();
    let blocks = source_drawing
        .blocks
        .iter()
        .map(|block| (block.id, block_ids[&block.id]))
        .collect::<BTreeMap<_, _>>();
    let layouts = source_drawing
        .layouts
        .iter()
        .map(|layout| (layout.id, layout_ids[&layout.id]))
        .collect::<BTreeMap<_, _>>();

    let normalized_entities = source_drawing
        .entities
        .iter()
        .map(|entity| normalize_entity(entity, &layers, &layouts, &blocks))
        .collect::<Result<Vec<_>, _>>()?;
    let mut entity_ids = BTreeMap::new();
    let mut native_values = BTreeSet::new();
    let mut matched_prior = BTreeSet::new();
    for (index, (entity, semantic)) in source_drawing
        .entities
        .iter()
        .zip(&normalized_entities)
        .enumerate()
    {
        let native_handle = imported.entity_identifier(entity.id).map(|identifier| {
            if native_values.insert(identifier.value.as_str()) {
                Ok(identifier.value.as_str())
            } else {
                Err(invalid_import())
            }
        });
        let native_handle = native_handle.transpose()?;
        let prior_id = reconcile_entity(
            semantic,
            native_handle,
            previous,
            &matched_prior,
            same_provider,
        )?;
        let identity = if let Some(id) = prior_id {
            matched_prior.insert(id);
            id
        } else {
            let key = entity_identity_key(semantic, native_handle, index)?;
            semantic_id(source, b"entity", &key)
        };
        entity_ids.insert(entity.id, identity);
    }
    if !same_provider && !previous.is_empty() && matched_prior.len() != previous.len() {
        return Err(invalid_import());
    }

    let mut drawing = source_drawing.clone();
    drawing.id = drawing_id;
    for layer in &mut drawing.layers {
        layer.id = remap(&layers, layer.id)?;
    }
    for entity in &mut drawing.entities {
        entity.id = remap(&entity_ids, entity.id)?;
        entity.layer = remap(&layers, entity.layer)?;
        entity.space = remap_space(&layouts, &entity.space)?;
        if let CadEntity::BlockReference(reference) = &mut entity.geometry {
            reference.definition = remap(&blocks, reference.definition)?;
        }
    }
    for block in &mut drawing.blocks {
        block.id = remap(&blocks, block.id)?;
        block.entities = block
            .entities
            .iter()
            .map(|entity| remap(&entity_ids, *entity))
            .collect::<Result<Vec<_>, _>>()?;
    }
    for layout in &mut drawing.layouts {
        layout.id = remap(&layouts, layout.id)?;
        for (index, viewport) in layout.viewports.iter_mut().enumerate() {
            let key = serde_json::to_vec(&(&viewport.center, &viewport.height, index))
                .map_err(|_| invalid_import())?;
            viewport.id = semantic_id(layout.id, b"viewport", &key);
        }
    }
    for (index, reference) in drawing.external_references.iter_mut().enumerate() {
        let key = serde_json::to_vec(&(&reference.target, index)).map_err(|_| invalid_import())?;
        reference.id = semantic_id(drawing.id, b"external-reference", &key);
        if let Some(target) = reference.drawing {
            if target != source_drawing.id {
                return Err(invalid_import());
            }
            reference.drawing = Some(drawing.id);
        }
    }
    let identifiers = imported
        .entity_identifiers()
        .iter()
        .map(|identifier| {
            Ok(CadProviderEntityIdentifier {
                entity: remap(&entity_ids, identifier.entity)?,
                value: identifier.value.clone(),
            })
        })
        .collect::<Result<Vec<_>, CadProviderError>>()?;
    Ok((drawing, identifiers))
}

#[path = "normalize/identity.rs"]
mod identity;
use identity::{entity_identity_key, normalize_entity, reconcile_entity};

/// Creates deterministic IDs for named drawing members and duplicate names.
///
/// # Parameters
/// - `source`: Stable source artifact identity.
/// - `kind`: Identity category such as layer or block.
/// - `names`: Authored names in source order.
///
/// # Returns
/// Provider-ID to Scriberig-ID mapping for the named members.
fn named_ids<'a>(
    source: EntityId,
    kind: &[u8],
    named_members: impl Iterator<Item = (EntityId, &'a str)>,
) -> BTreeMap<EntityId, EntityId> {
    let mut occurrences = BTreeMap::<String, usize>::new();
    named_members
        .map(|(provider_id, name)| {
            let occurrence = occurrences.entry(name.to_owned()).or_default();
            let mut key = name.as_bytes().to_vec();
            key.push(0);
            key.extend_from_slice(&occurrence.to_be_bytes());
            *occurrence += 1;
            (provider_id, semantic_id(source, kind, &key))
        })
        .collect()
}

/// Resolves one provider ID through its validated semantic remapping.
///
/// # Parameters
/// - `mapping`: Provider identity to Scriberig identity map.
/// - `provider_id`: Provider-created temporary identity.
///
/// # Returns
/// The Scriberig-owned identity.
///
/// # Errors
/// Returns invalid provenance when the provider identity is not in the map.
fn remap(
    mapping: &BTreeMap<EntityId, EntityId>,
    provider_id: EntityId,
) -> Result<EntityId, CadProviderError> {
    mapping
        .get(&provider_id)
        .copied()
        .ok_or_else(invalid_import)
}

/// Remaps a drawing-space reference to a normalized layout identity.
///
/// # Parameters
/// - `layouts`: Provider-to-Scriberig layout mapping.
/// - `space`: Source drawing-space value.
///
/// # Returns
/// A drawing-space value containing no provider-created layout ID.
///
/// # Errors
/// Returns invalid provenance when the referenced layout is absent.
fn remap_space(
    layouts: &BTreeMap<EntityId, EntityId>,
    space: &DrawingSpace,
) -> Result<DrawingSpace, CadProviderError> {
    match space {
        DrawingSpace::Model => Ok(DrawingSpace::Model),
        DrawingSpace::Paper(layout) => Ok(DrawingSpace::Paper(remap(layouts, *layout)?)),
        DrawingSpace::Named(name) => Ok(DrawingSpace::Named(name.clone())),
    }
}

/// Derives one namespaced semantic identity from the source artifact.
///
/// # Parameters
/// - `source`: Stable Scriberig source artifact identity.
/// - `kind`: Semantic identity category.
/// - `key`: Deterministic category-specific identity evidence.
///
/// # Returns
/// Scriberig-owned entity identity independent of provider-generated IDs.
fn semantic_id(source: EntityId, kind: &[u8], key: &[u8]) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-project-dwg-v1\0");
    hasher.update(source.as_bytes());
    hasher.update(kind);
    hasher.update(key);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Returns the closed invalid-provider-import provenance category.
///
/// # Returns
/// Provider error with the closed invalid-provenance category.
fn invalid_import() -> CadProviderError {
    CadProviderError {
        kind: CadProviderErrorKind::InvalidImportProvenance,
    }
}

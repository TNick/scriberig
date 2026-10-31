//! Matches provider entities to prior project identities using validated evidence.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::EntityId;
use scriberig_domain_cad::{CadEntity, CadProviderError};
use scriberig_domain_dxf::DxfPreviousEntity;

use super::{invalid_import, remap, remap_space};

/// Remaps provider-owned references in one entity before identity matching.
///
/// # Parameters
/// - `entity`: Provider semantic entity.
/// - `layers`: Provider-to-project layer identity map.
/// - `layouts`: Provider-to-project layout identity map.
/// - `blocks`: Provider-to-project block identity map.
///
/// # Returns
/// Entity semantics whose references use project-owned identities.
///
/// # Errors
/// Returns invalid provenance when a reference targets an unknown provider ID.
pub(super) fn normalize_entity(
    entity: &scriberig_domain_cad::DrawingEntity,
    layers: &BTreeMap<EntityId, EntityId>,
    layouts: &BTreeMap<EntityId, EntityId>,
    blocks: &BTreeMap<EntityId, EntityId>,
) -> Result<scriberig_domain_cad::DrawingEntity, CadProviderError> {
    let mut normalized = entity.clone();
    normalized.layer = remap(layers, entity.layer)?;
    normalized.space = remap_space(layouts, &entity.space)?;
    if let CadEntity::BlockReference(reference) = &mut normalized.geometry {
        reference.definition = remap(blocks, reference.definition)?;
    }
    Ok(normalized)
}

/// Finds one prior entity with safe identity continuity evidence.
///
/// Exact typed semantics take precedence. A unique same-kind/layer/space/style
/// match then preserves identity across an edited entity or provider change.
///
/// # Parameters
/// - `current`: Current entity with normalized layer/space/block references.
/// - `native_handle`: Current provider handle when available.
/// - `previous`: Prior project semantic entities.
/// - `matched`: Prior identities already assigned in this import.
/// - `same_provider`: Whether native handles share provider lineage.
///
/// # Returns
/// The prior semantic identity when one unique match is supported.
///
/// # Errors
/// Returns invalid provenance when evidence is ambiguous.
pub(super) fn reconcile_entity(
    current: &scriberig_domain_cad::DrawingEntity,
    native_handle: Option<&str>,
    previous: &[DxfPreviousEntity],
    matched: &BTreeSet<EntityId>,
    same_provider: bool,
) -> Result<Option<EntityId>, CadProviderError> {
    if same_provider && let Some(handle) = native_handle {
        let handle_matches = previous
            .iter()
            .filter(|candidate| {
                !matched.contains(&candidate.semantic.id)
                    && candidate.native_handle.as_deref() == Some(handle)
            })
            .map(|candidate| candidate.semantic.id)
            .collect::<Vec<_>>();
        if handle_matches.len() > 1 {
            return Err(invalid_import());
        }
        if let Some(id) = handle_matches.first() {
            return Ok(Some(*id));
        }
    }

    let exact = previous
        .iter()
        .filter(|candidate| {
            !matched.contains(&candidate.semantic.id)
                && same_semantics(current, &candidate.semantic)
        })
        .map(|candidate| candidate.semantic.id)
        .collect::<Vec<_>>();
    if exact.len() > 1 {
        return Err(invalid_import());
    }
    if let Some(id) = exact.first() {
        return Ok(Some(*id));
    }

    if !same_provider {
        return Ok(None);
    }
    let structural = previous
        .iter()
        .filter(|candidate| {
            !matched.contains(&candidate.semantic.id)
                && same_structure(current, &candidate.semantic)
        })
        .map(|candidate| candidate.semantic.id)
        .collect::<Vec<_>>();
    if structural.len() > 1 {
        return Err(invalid_import());
    }
    Ok(structural.first().copied())
}

/// Compares exact typed semantics while ignoring the semantic entity ID.
///
/// # Parameters
/// - `left`: Current normalized entity.
/// - `right`: Prior normalized entity.
///
/// # Returns
/// Whether geometry, layer, space, and style are equal.
fn same_semantics(
    left: &scriberig_domain_cad::DrawingEntity,
    right: &scriberig_domain_cad::DrawingEntity,
) -> bool {
    left.geometry == right.geometry
        && left.layer == right.layer
        && left.space == right.space
        && left.style == right.style
}

/// Compares entity type and stable ownership context for edit continuity.
///
/// # Parameters
/// - `left`: Current normalized entity.
/// - `right`: Prior normalized entity.
///
/// # Returns
/// Whether kind, layer, drawing space, and style agree.
fn same_structure(
    left: &scriberig_domain_cad::DrawingEntity,
    right: &scriberig_domain_cad::DrawingEntity,
) -> bool {
    entity_kind(&left.geometry) == entity_kind(&right.geometry)
        && left.layer == right.layer
        && left.space == right.space
        && left.style == right.style
}

/// Returns the adapter-neutral kind of one CAD geometry value.
///
/// # Parameters
/// - `geometry`: Shared CAD entity geometry.
///
/// # Returns
/// Stable CAD geometry kind used only for reconciliation bucketing.
fn entity_kind(geometry: &CadEntity) -> u8 {
    match geometry {
        CadEntity::Line { .. } => 0,
        CadEntity::Polyline { .. } => 1,
        CadEntity::Circle { .. } => 2,
        CadEntity::Arc { .. } => 3,
        CadEntity::Text { .. } => 4,
        CadEntity::Dimension { .. } => 5,
        CadEntity::BlockReference(_) => 6,
    }
}

/// Derives identity evidence for a new or unreconciled entity.
///
/// # Parameters
/// - `semantic`: Current normalized entity.
/// - `native_handle`: Provider-owned handle, if present.
/// - `index`: Source order for deterministic disambiguation without a handle.
///
/// # Returns
/// Namespaced key bytes for source-derived semantic identity.
///
/// # Errors
/// Returns invalid provenance when semantic evidence cannot be encoded.
pub(super) fn entity_identity_key(
    semantic: &scriberig_domain_cad::DrawingEntity,
    native_handle: Option<&str>,
    index: usize,
) -> Result<Vec<u8>, CadProviderError> {
    if let Some(handle) = native_handle {
        return Ok([b"native\0".as_slice(), handle.as_bytes()].concat());
    }
    let mut key = b"semantic\0".to_vec();
    key.extend_from_slice(
        &serde_json::to_vec(&(
            &semantic.geometry,
            &semantic.layer,
            &semantic.space,
            &semantic.style,
            index,
        ))
        .map_err(|_| invalid_import())?,
    );
    Ok(key)
}

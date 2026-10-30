//! Indexes prior CAD entities as evidence for stable semantic identity reconciliation.

use std::collections::{BTreeMap, HashMap, HashSet};

use scriberig_core::EntityId;
use scriberig_domain_cad::{CadEntity, DrawingSpace};

use crate::DxfPreviousEntity;

/// Resolves prior entity identities through indexed external and structural evidence.
pub(super) struct DxfEntityReconciler<'a> {
    /// Prior entity payloads referenced by all index entries.
    previous: &'a [DxfPreviousEntity],
    /// External native handles mapped to prior entity indexes.
    by_handle: HashMap<String, Vec<usize>>,
    /// Exact layer and semantic geometry mapped to prior entity indexes.
    by_structure: BTreeMap<(EntityId, Vec<u8>), Vec<usize>>,
    /// Prior entities already reconciled with this source import.
    used: HashSet<usize>,
}

impl<'a> DxfEntityReconciler<'a> {
    /// Builds one handle and structural index for the prior drawing.
    ///
    /// # Parameters
    /// - `previous`: Prior semantic entities from the exact previous snapshot.
    ///
    /// # Returns
    /// Reconciler whose lookups are bounded by matching evidence rather than all entities.
    pub(super) fn new(previous: &'a [DxfPreviousEntity]) -> Self {
        let mut by_handle = HashMap::<String, Vec<usize>>::new();
        let mut by_structure = BTreeMap::<(EntityId, Vec<u8>), Vec<usize>>::new();
        for (index, entity) in previous.iter().enumerate() {
            if let Some(handle) = &entity.native_handle {
                by_handle.entry(handle.clone()).or_default().push(index);
            }
            if entity.semantic.space == DrawingSpace::Model
                && let Ok(geometry) = serde_json::to_vec(&entity.semantic.geometry)
            {
                by_structure
                    .entry((entity.semantic.layer, geometry))
                    .or_default()
                    .push(index);
            }
        }
        Self {
            previous,
            by_handle,
            by_structure,
            used: HashSet::new(),
        }
    }

    /// Resolves a unique prior semantic identity from handle or exact structure evidence.
    ///
    /// # Parameters
    /// - `native_handle`: External DXF source handle.
    /// - `layer`: Current semantic layer identity.
    /// - `geometry`: Current semantic geometry.
    ///
    /// # Returns
    /// Prior semantic identity for unique evidence, `None` when absent, or an error when ambiguous.
    ///
    /// # Errors
    /// Returns an error when multiple unused prior entities share the same evidence.
    pub(super) fn resolve(
        &mut self,
        native_handle: &str,
        layer: EntityId,
        geometry: &CadEntity,
    ) -> Result<Option<EntityId>, ()> {
        if let Some(indexes) = self.by_handle.get(native_handle) {
            if let Some(index) = self.unique_unused(indexes)? {
                return Ok(self.mark_used(index));
            }
        }
        let Ok(geometry_key) = serde_json::to_vec(geometry) else {
            return Ok(None);
        };
        let Some(indexes) = self.by_structure.get(&(layer, geometry_key)) else {
            return Ok(None);
        };
        let Some(index) = self.unique_unused(indexes)? else {
            return Ok(None);
        };
        Ok(self.mark_used(index))
    }

    /// Finds the only unused index in one evidence bucket.
    ///
    /// # Parameters
    /// - `indexes`: Prior entity indexes sharing the same evidence.
    ///
    /// # Returns
    /// The unique unused index, if exactly one exists.
    ///
    /// # Errors
    /// Returns `Err` when multiple unused indexes share the evidence.
    fn unique_unused(&self, indexes: &[usize]) -> Result<Option<usize>, ()> {
        let mut unused = indexes
            .iter()
            .copied()
            .filter(|index| !self.used.contains(index));
        let Some(index) = unused.next() else {
            return Ok(None);
        };
        if unused.next().is_some() {
            return Err(());
        }
        Ok(Some(index))
    }

    /// Marks one prior entity as reconciled and returns its semantic identity.
    ///
    /// # Parameters
    /// - `index`: Prior entity index selected by unique evidence.
    ///
    /// # Returns
    /// Prior semantic entity identity.
    fn mark_used(&mut self, index: usize) -> Option<EntityId> {
        self.used.insert(index);
        self.previous.get(index).map(|entity| entity.semantic.id)
    }
}

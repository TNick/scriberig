//! Stores snapshot-local spreadsheet membership and coordinate indexes.

use std::collections::BTreeSet;

use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

use crate::{CellRange, CollectionError};

/// Identifies one cell revision in a worksheet membership index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CellIndexEntry {
    /// Stable semantic cell entity identity.
    pub entity: EntityId,
    /// One-based worksheet row.
    pub row: u32,
    /// One-based worksheet column.
    pub column: u32,
}

/// Provides deterministic bounded access to snapshot-local cell membership.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CellIndex {
    /// Entries ordered by row and then column.
    entries: Vec<CellIndexEntry>,
}

impl CellIndex {
    /// Builds a validated row-major index from semantic cell identities.
    ///
    /// # Parameters
    /// - `entries`: Cell entity identities and one-based coordinates.
    ///
    /// # Returns
    /// An index sorted by row and column.
    ///
    /// # Errors
    /// Returns an error when coordinates or identities are invalid or duplicated.
    pub fn new(entries: impl IntoIterator<Item = CellIndexEntry>) -> Result<Self, CollectionError> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_by_key(|entry| (entry.row, entry.column));
        let index = Self { entries };
        index.validate()?;
        Ok(index)
    }

    /// Returns the number of indexed semantic cells.
    ///
    /// # Returns
    /// Number of membership entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether the membership index is empty.
    ///
    /// # Returns
    /// True when no cell entity is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Selects one bounded page and count from an exact membership index.
    ///
    /// Range access seeks to the first matching row in the ordered index,
    /// then examines only index entries in those rows. Cell revisions are not
    /// loaded by this method.
    ///
    /// # Parameters
    /// - `range`: Optional inclusive row and column bounds.
    /// - `offset`: Number of matching cells to skip.
    /// - `limit`: Maximum entries returned, from one through five hundred.
    ///
    /// # Returns
    /// Selected stable handles, total matching count, and continuation state.
    ///
    /// # Errors
    /// Returns an error for a malformed index, range, or page limit.
    pub fn page(
        &self,
        range: Option<&CellRange>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<CellIndexEntry>, usize, bool), CollectionError> {
        if !(1..=500).contains(&limit) {
            return Err(CollectionError::InvalidLimit);
        }
        self.validate()?;
        if let Some(range) = range {
            range.validate()?;
        }
        let (start, end) = range.map_or((0, self.entries.len()), |bounds| {
            (
                self.entries
                    .partition_point(|entry| entry.row < bounds.first_row),
                self.entries
                    .partition_point(|entry| entry.row <= bounds.last_row),
            )
        });
        let mut total = 0;
        let mut selected = Vec::with_capacity(limit);
        for entry in &self.entries[start..end] {
            if range.is_some_and(|bounds| !bounds.contains(entry.row, entry.column)) {
                continue;
            }
            if total >= offset && selected.len() < limit {
                selected.push(*entry);
            }
            total += 1;
        }
        let has_more = total > offset.saturating_add(selected.len());
        Ok((selected, total, has_more))
    }

    /// Checks serialized entries for coordinate order and identity uniqueness.
    ///
    /// # Returns
    /// Unit when every entry satisfies the index invariants.
    ///
    /// # Errors
    /// Returns `InvalidCellIndex` for invalid coordinates, duplicate identities,
    /// or entries that are not ordered by row and column.
    pub fn validate(&self) -> Result<(), CollectionError> {
        if self
            .entries
            .iter()
            .any(|entry| entry.row == 0 || entry.column == 0)
            || self
                .entries
                .windows(2)
                .any(|pair| (pair[0].row, pair[0].column) >= (pair[1].row, pair[1].column))
        {
            return Err(CollectionError::InvalidCellIndex);
        }
        let mut identities = BTreeSet::new();
        for entry in &self.entries {
            if !identities.insert(entry.entity) {
                return Err(CollectionError::InvalidCellIndex);
            }
        }
        Ok(())
    }
}

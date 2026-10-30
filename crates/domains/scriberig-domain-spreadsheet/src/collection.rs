//! Implements sparse deterministic cell storage and bounded range access.

use std::collections::BTreeMap;

use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

use crate::{Cell, CellId};

/// Default maximum number of cells returned by a page request.
pub const DEFAULT_CELL_PAGE_LIMIT: usize = 256;

/// Stores only authored or otherwise semantically meaningful cells.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CellCollection {
    /// Workbook that owns this collection.
    workbook: EntityId,
    /// Worksheet that owns this collection.
    worksheet: EntityId,
    #[serde(with = "cell_map_serde")]
    cells: BTreeMap<(u32, u32), Cell>,
}

/// Encodes coordinate-keyed cells as a portable ordered sequence.
mod cell_map_serde {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::Cell;

    /// Serializes cell records in deterministic row-major order.
    ///
    /// - cells: Coordinate-indexed semantic cells.
    /// - serializer: Serialization destination.
    ///
    /// Serialized cell sequence.
    ///
    /// Returns a serializer error if any cell cannot be encoded.
    pub(super) fn serialize<S>(
        cells: &BTreeMap<(u32, u32), Cell>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        cells.values().collect::<Vec<_>>().serialize(serializer)
    }

    /// Deserializes cells and rebuilds their coordinate index.
    ///
    /// - deserializer: Serialized cell sequence source.
    ///
    /// Coordinate-indexed semantic cells.
    ///
    /// Returns a deserializer error for duplicate coordinates.
    pub(super) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<(u32, u32), Cell>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cells = Vec::<Cell>::deserialize(deserializer)?;
        let mut indexed = BTreeMap::new();
        for cell in cells {
            if indexed.insert((cell.row, cell.column), cell).is_some() {
                return Err(serde::de::Error::custom("duplicate cell coordinate"));
            }
        }
        Ok(indexed)
    }
}

impl CellCollection {
    /// Creates a sparse collection from cells belonging to one worksheet.
    ///
    /// # Parameters
    /// - `workbook`: Stable workbook semantic identity.
    /// - `worksheet`: Stable worksheet semantic identity.
    /// - `cells`: Authored or otherwise meaningful cells.
    ///
    /// # Returns
    /// A validated sparse collection.
    ///
    /// # Errors
    /// Returns an error for invalid coordinates, wrong worksheet, or duplicate cell positions.
    pub fn new(
        workbook: EntityId,
        worksheet: EntityId,
        cells: impl IntoIterator<Item = Cell>,
    ) -> Result<Self, CollectionError> {
        let mut collection = Self {
            workbook,
            worksheet,
            cells: BTreeMap::new(),
        };
        for mut cell in cells {
            validate_coordinates(cell.row, cell.column)?;
            if cell.worksheet != worksheet {
                return Err(CollectionError::WrongWorksheet);
            }
            cell.id = CellId::new(workbook, worksheet, cell.row, cell.column);
            if collection
                .cells
                .insert((cell.row, cell.column), cell)
                .is_some()
            {
                return Err(CollectionError::DuplicateCoordinate);
            }
        }
        Ok(collection)
    }

    /// Returns the number of stored non-empty or authored cells.
    ///
    /// # Returns
    /// Number of authored sparse cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Reports whether the collection has no stored cells.
    ///
    /// # Returns
    /// True when no semantic cell is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Iterates stored cells in deterministic row-major order.
    ///
    /// This method supports explicit adapter import/export jobs. Product reads
    /// should use the bounded page method instead.
    ///
    /// # Returns
    /// Stored cells without materializing empty grid positions.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Cell> {
        self.cells.values()
    }

    /// Reads at most limit cells in deterministic row-major order.
    ///
    /// # Parameters
    /// - `range`: Optional exact rectangular range.
    /// - `offset`: Number of matching cells to skip.
    /// - `limit`: Maximum number of returned cells; must be between 1 and 500.
    ///
    /// # Returns
    /// Matching sparse cells and whether another page exists.
    ///
    /// # Errors
    /// Returns an error for invalid ranges or page limits.
    pub fn page(
        &self,
        range: Option<&CellRange>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<&Cell>, bool), CollectionError> {
        if !(1..=500).contains(&limit) {
            return Err(CollectionError::InvalidLimit);
        }
        if let Some(range) = range {
            range.validate()?;
        }
        let mut matched: Box<dyn Iterator<Item = &Cell> + '_> = if let Some(area) = range {
            Box::new(
                self.cells
                    .range((area.first_row, 0)..=(area.last_row, u32::MAX))
                    .filter(move |((_, column), _)| {
                        (*column >= area.first_column) && (*column <= area.last_column)
                    })
                    .map(|(_, cell)| cell),
            )
        } else {
            Box::new(self.cells.values())
        };
        let selected = matched
            .by_ref()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let has_more = matched.next().is_some();
        Ok((selected, has_more))
    }

    /// Replaces one cell and returns the exact locally changed cell identity.
    ///
    /// # Parameters
    /// - `cell`: Replacement cell with its semantic value.
    ///
    /// # Returns
    /// The stable identity of the changed cell.
    ///
    /// # Errors
    /// Returns an error for invalid coordinates or a mismatched worksheet.
    pub fn set(&mut self, mut cell: Cell) -> Result<CellId, CollectionError> {
        validate_coordinates(cell.row, cell.column)?;
        if cell.worksheet != self.worksheet {
            return Err(CollectionError::WrongWorksheet);
        }
        cell.id = CellId::new(self.workbook, self.worksheet, cell.row, cell.column);
        self.cells.insert((cell.row, cell.column), cell.clone());
        Ok(cell.id)
    }
}

/// Represents an inclusive one-based cell range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellRange {
    /// First included row.
    pub first_row: u32,
    /// First included column.
    pub first_column: u32,
    /// Last included row.
    pub last_row: u32,
    /// Last included column.
    pub last_column: u32,
}

impl CellRange {
    /// Validates that all coordinates are positive and the bounds are ordered.
    ///
    /// # Parameters
    /// - `self`: Range to validate.
    ///
    /// # Returns
    /// Unit for a valid range.
    ///
    /// # Errors
    /// Returns an error for zero or reversed coordinates.
    pub fn validate(self) -> Result<(), CollectionError> {
        validate_coordinates(self.first_row, self.first_column)?;
        validate_coordinates(self.last_row, self.last_column)?;
        if self.first_row > self.last_row || self.first_column > self.last_column {
            return Err(CollectionError::InvalidRange);
        }
        Ok(())
    }

    /// Reports whether a cell coordinate lies inside this inclusive range.
    ///
    /// # Parameters
    /// - `self`: Range to test.
    /// - `row`: One-based cell row.
    /// - `column`: One-based cell column.
    ///
    /// # Returns
    /// True when the cell is within the range.
    #[must_use]
    pub fn contains(self, row: u32, column: u32) -> bool {
        (self.first_row..=self.last_row).contains(&row)
            && (self.first_column..=self.last_column).contains(&column)
    }
}

/// Reports invalid sparse-cell or range requests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionError {
    /// A coordinate is zero; cell coordinates are one-based.
    InvalidCoordinate,
    /// Range start follows its end.
    InvalidRange,
    /// A page size is outside the supported range.
    InvalidLimit,
    /// Two cells occupy the same coordinate.
    DuplicateCoordinate,
    /// A cell belongs to another worksheet.
    WrongWorksheet,
    /// A serialized cell membership index has invalid or duplicate entries.
    InvalidCellIndex,
}

/// Validates one-based row and column coordinates.
///
/// # Parameters
/// - `row`: One-based row coordinate.
/// - `column`: One-based column coordinate.
///
/// # Returns
/// Unit when both coordinates are valid.
///
/// # Errors
/// Returns an error when either coordinate is zero.
fn validate_coordinates(row: u32, column: u32) -> Result<(), CollectionError> {
    if row == 0 || column == 0 {
        return Err(CollectionError::InvalidCoordinate);
    }
    Ok(())
}

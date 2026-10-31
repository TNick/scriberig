//! Defines the canonical semantic vocabulary for spreadsheet artifacts.

use crate::CellCollection;
use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

/// Identifies a cell independently of row labels and workbook display names.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CellId(EntityId);

impl CellId {
    /// Derives a stable cell identity from its workbook, sheet, and coordinates.
    ///
    /// # Parameters
    /// - `workbook`: Stable workbook semantic identity.
    /// - `worksheet`: Stable worksheet semantic identity.
    /// - `row`: One-based row coordinate.
    /// - `column`: One-based column coordinate.
    ///
    /// # Returns
    /// A deterministic cell identity.
    #[must_use]
    pub fn new(workbook: EntityId, worksheet: EntityId, row: u32, column: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"scriberig-spreadsheet-cell-v1\0");
        hasher.update(workbook.as_bytes());
        hasher.update(worksheet.as_bytes());
        hasher.update(&row.to_be_bytes());
        hasher.update(&column.to_be_bytes());
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
        Self(EntityId::from_bytes(bytes))
    }

    /// Returns the semantic entity identity represented by this cell.
    #[must_use]
    pub const fn entity(self) -> EntityId {
        self.0
    }
}

/// Represents a workbook containing ordered worksheets and metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Workbook {
    /// Stable workbook semantic identity.
    pub id: EntityId,
    /// Ordered worksheet semantic identities.
    pub worksheets: Vec<EntityId>,
    /// Workbook-scoped defined names.
    pub defined_names: Vec<DefinedName>,
}

/// Represents one worksheet independently of its position in the workbook.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Worksheet {
    /// Stable worksheet semantic identity.
    pub id: EntityId,
    /// Owning workbook semantic identity.
    pub workbook: EntityId,
    /// Authored worksheet name.
    pub name: String,
    /// Sparse addressable cells; absent cells are semantically blank.
    pub cells: CellCollection,
    /// Bounded authored cell extent when the adapter can determine one.
    pub used_range: Option<Range>,
    /// Tables present on the worksheet.
    pub tables: Vec<Table>,
}

/// Represents an addressable non-empty spreadsheet cell.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Stable semantic cell identity.
    pub id: CellId,
    /// Owning worksheet identity.
    pub worksheet: EntityId,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
    /// Authored cell value or formula.
    pub value: CellValue,
    /// Optional style reference independent of semantic value.
    pub style: Option<CellStyleRef>,
}

/// Represents a typed cell value without treating cached formula output as truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CellValue {
    /// Explicit blank value.
    Blank,
    /// Authored UTF-8 text.
    Text(String),
    /// Exact numeric text retained from the source representation.
    Number(String),
    /// Boolean value.
    Boolean(bool),
    /// Authored spreadsheet error value.
    Error(String),
    /// Authored formula and optional non-authoritative cached result.
    Formula(Formula),
}

/// Represents authored formula syntax separately from calculated values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Formula {
    /// Formula text including its authored leading equals sign when present.
    pub expression: String,
    /// Cached value retained as advisory source data.
    pub cached_value: Option<Box<CellValue>>,
}

/// Represents a style identity separate from cell semantic content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CellStyleRef {
    /// Adapter-neutral style identity.
    pub key: String,
    /// Style source adapter identifier.
    pub adapter: String,
}

/// Represents a rectangular area in one worksheet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Range {
    /// Worksheet containing the range.
    pub worksheet: EntityId,
    /// First one-based row.
    pub first_row: u32,
    /// First one-based column.
    pub first_column: u32,
    /// Last one-based row.
    pub last_row: u32,
    /// Last one-based column.
    pub last_column: u32,
}

/// Represents a workbook or worksheet defined name.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DefinedName {
    /// Authored name.
    pub name: String,
    /// Formula or range expression authored by the workbook.
    pub expression: String,
    /// Optional worksheet scope.
    pub scope: Option<EntityId>,
}

/// Represents a named rectangular data table.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// Stable table semantic identity.
    pub id: EntityId,
    /// Authored table name.
    pub name: String,
    /// Semantic range occupied by the table.
    pub range: Range,
}

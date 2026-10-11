//! Defines semantic explorer rows without inventing unsupported relation trees.

use crate::selection::EntitySelection;

/// One explorer row backed by a stable semantic selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerRow {
    /// Stable row selection.
    pub selection: EntitySelection,
    /// Safe compact label.
    pub label: String,
    /// Whether the row can be activated for inspection.
    pub inspectable: bool,
}

/// Explorer model containing only explicitly known semantic rows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExplorerModel {
    /// Optional semantic root row.
    pub root: Option<ExplorerRow>,
    /// Rows returned by a future public list/follow operation.
    pub rows: Vec<ExplorerRow>,
}

impl ExplorerModel {
    /// Creates an explorer rooted at an explicitly entered entity.
    #[must_use]
    pub fn from_selection(selection: EntitySelection) -> Self {
        let label = selection.entity.to_string();
        Self {
            root: Some(ExplorerRow {
                selection,
                label,
                inspectable: true,
            }),
            rows: Vec::new(),
        }
    }
}

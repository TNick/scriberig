//! Defines format-independent, sparse spreadsheet semantics and bounded reads.

mod cell_index;
mod collection;
mod formula_references;
mod registration;
mod schema;

pub use cell_index::{CellIndex, CellIndexEntry};
pub use collection::{CellCollection, CellRange, CollectionError, DEFAULT_CELL_PAGE_LIMIT};
pub use formula_references::{CellAddress, FormulaReference};
pub use registration::spreadsheet_schema;
pub use schema::{
    Cell, CellId, CellStyleRef, CellValue, DefinedName, Formula, Range, Table, Workbook, Worksheet,
};

#[cfg(test)]
mod tests {
    use scriberig_core::EntityId;

    use super::{Cell, CellCollection, CellId, CellIndex, CellIndexEntry, CellRange, CellValue};

    /// Creates one test cell at a worksheet coordinate.
    ///
    /// # Parameters
    /// - `worksheet`: Owning worksheet identity.
    /// - `row`: One-based cell row.
    /// - `column`: One-based cell column.
    /// - `text`: Cell text value.
    ///
    /// # Returns
    /// A cell with a deterministic coordinate-derived identity.
    fn cell(worksheet: EntityId, row: u32, column: u32, text: String) -> Cell {
        Cell {
            id: CellId::new(EntityId::default(), worksheet, row, column),
            worksheet,
            row,
            column,
            value: CellValue::Text(text),
            style: None,
        }
    }

    /// Verifies sparse storage pages a large logical sheet without grid expansion.
    #[test]
    fn large_sheet_reads_only_requested_page_and_range() {
        let workbook = EntityId::new();
        let worksheet = EntityId::new();
        let cells = (1..=30_000).map(|row| cell(worksheet, row, 2, row.to_string()));
        let collection =
            CellCollection::new(workbook, worksheet, cells).expect("valid sparse sheet");
        assert_eq!(collection.len(), 30_000);
        let (page, has_more) = collection.page(None, 10, 20).expect("bounded page");
        assert_eq!(page.len(), 20);
        assert!(has_more);
        assert_eq!(page.first().map(|cell| cell.row), Some(11));
        assert_eq!(page.last().map(|cell| cell.row), Some(30));
        let (second_page, second_has_more) =
            collection.page(None, 30, 20).expect("second bounded page");
        assert_eq!(second_page.len(), 20);
        assert!(second_has_more);
        assert_eq!(second_page.first().map(|cell| cell.row), Some(31));
        assert_eq!(second_page.last().map(|cell| cell.row), Some(50));
        let (range, has_more) = collection
            .page(
                Some(&CellRange {
                    first_row: 20_000,
                    first_column: 2,
                    last_row: 20_010,
                    last_column: 2,
                }),
                0,
                20,
            )
            .expect("bounded range");
        assert_eq!(range.len(), 11);
        assert!(!has_more);
        assert!(
            range
                .iter()
                .all(|cell| (20_000..=20_010).contains(&cell.row))
        );
    }

    /// Verifies snapshot membership paging selects stable handles from a large index.
    #[test]
    fn large_cell_index_pages_and_filters_ranges_without_loading_cell_values() {
        let entries = (1..=30_000).map(|row| CellIndexEntry {
            entity: EntityId::new(),
            row,
            column: 2,
        });
        let index = CellIndex::new(entries).expect("valid membership index");
        let (page, total, has_more) = index.page(None, 10, 20).expect("bounded page");
        assert_eq!(total, 30_000);
        assert_eq!(page.len(), 20);
        assert_eq!(page.first().map(|entry| entry.row), Some(11));
        assert_eq!(page.last().map(|entry| entry.row), Some(30));
        assert!(has_more);
        let range = CellRange {
            first_row: 20_000,
            first_column: 2,
            last_row: 20_010,
            last_column: 2,
        };
        let (selected, total, has_more) =
            index.page(Some(&range), 0, 20).expect("bounded range page");
        assert_eq!(total, 11);
        assert_eq!(selected.len(), 11);
        assert!(!has_more);
    }

    /// Verifies one value update keeps all unrelated semantic cell identities.
    #[test]
    fn changing_one_cell_yields_one_local_change() {
        let workbook = EntityId::new();
        let worksheet = EntityId::new();
        let mut collection = CellCollection::new(
            workbook,
            worksheet,
            [
                cell(worksheet, 3, 1, "10".into()),
                cell(worksheet, 3, 2, "=A3*2".into()),
            ],
        )
        .expect("valid sheet");
        let before = collection.page(None, 0, 10).expect("page").0;
        let unchanged_id = before[0].id;
        let changed_id = before[1].id;
        let mut replacement = cell(worksheet, 3, 2, "=A3*3".into());
        replacement.value = CellValue::Formula(crate::Formula {
            expression: "=A3*3".into(),
            cached_value: None,
        });
        assert_eq!(collection.set(replacement), Ok(changed_id));
        let after = collection.page(None, 0, 10).expect("page").0;
        assert_eq!(after[0].id, unchanged_id);
        assert_eq!(after[1].id, changed_id);
        assert_eq!(
            after[1].value,
            CellValue::Formula(crate::Formula {
                expression: "=A3*3".into(),
                cached_value: None,
            })
        );
    }

    /// Verifies a rectangular range excludes out-of-range columns on every row.
    #[test]
    fn range_query_filters_columns_across_all_included_rows() {
        let workbook = EntityId::new();
        let worksheet = EntityId::new();
        let collection = CellCollection::new(
            workbook,
            worksheet,
            [
                cell(worksheet, 1, 1, "inside-a".into()),
                cell(worksheet, 1, 3, "outside-right".into()),
                cell(worksheet, 2, 2, "inside-b".into()),
                cell(worksheet, 2, 4, "outside-right-later-row".into()),
            ],
        )
        .expect("valid sparse sheet");
        let (selected, more) = collection
            .page(
                Some(&CellRange {
                    first_row: 1,
                    first_column: 1,
                    last_row: 2,
                    last_column: 2,
                }),
                0,
                10,
            )
            .expect("rectangular range");
        assert_eq!(selected.len(), 2);
        assert!(!more);
        assert!(selected.iter().all(|value| value.column <= 2));
    }
}

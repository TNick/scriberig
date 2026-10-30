//! Registers stable spreadsheet entity and relationship names.

use scriberig_core::ObjectType;
use scriberig_schema::{DomainDescriptor, DomainId, SchemaVersion};

/// Entity kinds supplied by the format-independent spreadsheet domain.
const ENTITY_TYPES: &[&str] = &[
    "spreadsheet.workbook",
    "spreadsheet.worksheet",
    "spreadsheet.cell",
    "spreadsheet.table",
];

/// Ordered ownership relationships supplied by the spreadsheet domain.
const RELATION_TYPES: &[&str] = &[
    "spreadsheet.worksheets",
    "spreadsheet.cells",
    "spreadsheet.tables",
];

/// Returns the stable versioned spreadsheet semantic schema descriptor.
///
/// # Returns
/// Domain descriptor consumed by schema catalogs and adapters.
///
/// # Panics
/// Panics only if one of the static identifiers is invalid.
#[must_use]
pub fn spreadsheet_schema() -> DomainDescriptor {
    DomainDescriptor {
        id: DomainId::new("spreadsheet").expect("static spreadsheet domain id"),
        name: "Spreadsheet".to_owned(),
        schema_version: SchemaVersion::new(1),
        entity_types: ENTITY_TYPES
            .iter()
            .map(|name| ObjectType::new(*name).expect("static spreadsheet entity type"))
            .collect(),
        relation_types: RELATION_TYPES
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        capabilities: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::spreadsheet_schema;

    /// Verifies the registered spreadsheet schema is stable and contains its rich entity kinds.
    #[test]
    fn spreadsheet_schema_registers_workbook_cell_and_table_types() {
        let descriptor = spreadsheet_schema();
        assert_eq!(descriptor, spreadsheet_schema());
        assert_eq!(descriptor.schema_version.value(), 1);
        assert!(
            descriptor
                .entity_types
                .iter()
                .any(|kind| kind.as_str() == "spreadsheet.table")
        );
    }
}

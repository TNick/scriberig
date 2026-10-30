//! Declares candidate-producing methods in the shared rich-artifact contract.

macro_rules! shared_rich_artifact_edit_contract {
    () => {
        /// Proposes one QGIS layer-reference rename through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted exact-snapshot request context and Propose grants.
        /// - `_project`: Serialized owning project identity.
        /// - `_reference`: Serialized semantic QGIS layer-reference identity.
        /// - `_new_name`: New authored display name.
        ///
        /// # Returns
        /// Candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns unsupported-operation unless QGIS rename is exposed.
        fn propose_qgis_layer_rename(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _reference: &str,
            _new_name: &str,
        ) -> Result<scriberig_app::QgisLayerRenameCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "qgis.layer.rename".into(),
            })
        }

        /// Proposes a complete QGIS layer order through project-scoped Propose authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted exact-snapshot request context.
        /// - `_project`: Serialized owning project identity.
        /// - `_path`: Exact project-relative QGS/QGZ source path.
        /// - `_ordered_references`: Complete ordered semantic layer identities.
        ///
        /// # Returns
        /// Durable candidate metadata for shared validation and apply.
        ///
        /// # Errors
        /// Returns unsupported-operation unless QGIS ordering is exposed.
        fn propose_qgis_layer_order(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _path: &str,
            _ordered_references: &[String],
        ) -> Result<scriberig_app::QgisLayerOrderCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "qgis.layer_order.reorder".into(),
            })
        }

        /// Proposes an exact-base `GeoPackage` geometry candidate through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, exact snapshot, and scoped Propose grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_path`: Project-relative `GeoPackage` path.
        /// - `_layer`: Serialized semantic layer identity.
        /// - `_feature`: Serialized semantic feature identity.
        /// - `_geometry`: Typed replacement geometry in layer CRS.
        ///
        /// # Returns
        /// Durable candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns unsupported operation unless `GeoPackage` geometry edits are implemented.
        fn propose_geo_feature_geometry(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _path: &str,
            _layer: &str,
            _feature: &str,
            _geometry: &scriberig_domain_geo::Geometry,
        ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "geo.geometry.propose".into(),
            })
        }

        /// Proposes an exact-base feature attribute candidate through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, exact snapshot, and scoped Propose grants.
        /// - `_edit`: Exact project, source, feature, field, and typed replacement value.
        ///
        /// # Returns
        /// Durable candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns unsupported operation unless feature attribute edits are exposed.
        fn propose_geo_feature_attribute(
            &self,
            _context: &ApplicationContext,
            _edit: scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "geo.attribute.propose".into(),
            })
        }

        /// Proposes an exact-base spreadsheet edit through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_worksheet`: Serialized stable worksheet identity.
        /// - `_row`: One-based target row.
        /// - `_column`: One-based target column.
        /// - `_value`: Authored semantic cell value or formula.
        ///
        /// # Returns
        /// Durable candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns unsupported operation unless spreadsheet writes are implemented.
        fn propose_spreadsheet_cell_edit(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _worksheet: &str,
            _row: u32,
            _column: u32,
            _value: scriberig_protocol::SpreadsheetCellValue,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.propose".into(),
            })
        }

        /// Proposes a bounded multi-cell spreadsheet candidate.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_worksheet`: Serialized stable worksheet identity.
        /// - `_edits`: Typed updates for distinct worksheet coordinates.
        ///
        /// # Returns
        /// Durable candidate metadata for shared validation and review.
        ///
        /// # Errors
        /// Returns unsupported operation unless spreadsheet range writes are implemented.
        fn propose_spreadsheet_range_edit(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _worksheet: &str,
            _edits: Vec<scriberig_protocol::SpreadsheetCellEdit>,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.range.propose".to_owned(),
            })
        }
    };
}

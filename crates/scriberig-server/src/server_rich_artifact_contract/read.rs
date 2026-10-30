//! Declares read-only methods in the shared rich-artifact contract.

macro_rules! shared_rich_artifact_read_contract {
    () => {
        /// Finds paragraph text in an exact-snapshot semantic document.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `_request`: Typed project, snapshot, document, query, and page selection.
        ///
        /// # Returns
        /// Bounded semantic paragraph matches.
        ///
        /// # Errors
        /// Returns unsupported operation unless document search is exposed.
        fn find_document_paragraphs(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::DocumentParagraphSearchRequest,
        ) -> Result<scriberig_app::DocumentParagraphSearch, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.find".into(),
            })
        }

        /// Inspects one exact worksheet coordinate through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `_project`: Serialized project identity.
        /// - `_worksheet`: Serialized semantic worksheet identity.
        /// - `_row`: One-based row coordinate.
        /// - `_column`: One-based column coordinate.
        ///
        /// # Returns
        /// Typed semantic cell or an absent sparse blank coordinate.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes spreadsheet reads.
        fn inspect_spreadsheet_cell(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _worksheet: &str,
            _row: u32,
            _column: u32,
        ) -> Result<scriberig_app::SpreadsheetCellInspection, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.inspect".into(),
            })
        }

        /// Reads a workbook and ordered worksheet summaries at an exact snapshot.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `_project`: Serialized project identity.
        /// - `_workbook`: Serialized semantic workbook identity.
        ///
        /// # Returns
        /// Workbook metadata with worksheet names and used ranges.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes spreadsheet summaries.
        fn inspect_spreadsheet_workbook(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _workbook: &str,
        ) -> Result<scriberig_project::SpreadsheetWorkbookSummary, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.workbook.inspect".into(),
            })
        }

        /// Finds authored workbook cell values through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted exact-snapshot context with an Inspect grant.
        /// - `_request`: Typed project, snapshot, workbook, query, and page selection.
        ///
        /// # Returns
        /// Bounded semantic cell matches with stable identities.
        ///
        /// # Errors
        /// Returns unsupported operation unless spreadsheet search is exposed.
        fn find_spreadsheet_cells(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SpreadsheetCellSearchRequest,
        ) -> Result<scriberig_project::SpreadsheetCellSearch, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cell.find".into(),
            })
        }

        /// Lists authorized artifact summaries from one persisted project scan.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context.
        /// - `_project`: Serialized stable project identity.
        /// - `_offset`: Zero-based artifact page offset.
        /// - `_limit`: Maximum number of artifact summaries.
        ///
        /// # Returns
        /// A bounded project artifact page.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes artifacts.
        fn inspect_artifacts(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_app::ArtifactManifestPage, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "artifact.inspect".into(),
            })
        }

        /// Reads one bounded page of exact artifact member evidence.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context.
        /// - `_project`: Serialized stable project identity.
        /// - `_artifact`: Serialized stable artifact identity.
        /// - `_offset`: Zero-based member page offset.
        /// - `_limit`: Maximum number of member descriptors.
        ///
        /// # Returns
        /// A bounded exact artifact-member page.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes artifact members.
        fn inspect_artifact_members(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _artifact: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_app::ArtifactMembers, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "artifact.list_members".into(),
            })
        }

        /// Reads one authorized snapshot-bound spreadsheet cell page.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_worksheet`: Serialized stable worksheet identity.
        /// - `_range`: Optional inclusive rectangular coordinates.
        /// - `_offset`: Zero-based page offset.
        /// - `_limit`: Maximum returned cell count.
        ///
        /// # Returns
        /// Typed shared application cell page.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes spreadsheet reads.
        fn read_spreadsheet_cells(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _worksheet: &str,
            _range: Option<scriberig_protocol::SpreadsheetRange>,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_app::SpreadsheetCells, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.cells.read".into(),
            })
        }

        /// Reads a bounded `GeoPackage` feature page through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with the exact snapshot and grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_path`: Project-relative `GeoPackage` path from persisted scan evidence.
        /// - `_layer`: Serialized stable semantic layer identity.
        /// - `_offset`: Zero-based feature offset.
        /// - `_limit`: Maximum feature page size, at most one thousand.
        ///
        /// # Returns
        /// Snapshot-bound provider-neutral feature page.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes `GeoPackage` reads.
        fn read_geopackage_features(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _path: &str,
            _layer: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_app::GeoFeaturePage, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "geo.features.read".into(),
            })
        }

        /// Executes a bounded typed geospatial query under shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted exact-snapshot context with caller grants.
        /// - `_request`: Typed project, source, layer, predicate, and page bounds.
        ///
        /// # Returns
        /// Bounded matching features and exact source provenance.
        ///
        /// # Errors
        /// Returns unsupported operation unless geospatial queries are exposed.
        fn query_geospatial_features(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::GeoFeatureQuery,
        ) -> Result<scriberig_app::GeoFeatureQueryResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "geo.features.query".into(),
            })
        }

        /// Reads a bounded `GeoPackage` layer catalog through shared Inspect authority.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_path`: Project-relative `GeoPackage` source path.
        /// - `_offset`: Zero-based layer offset.
        /// - `_limit`: Maximum returned layer count.
        ///
        /// # Returns
        /// Snapshot-bound dataset and layer catalog page.
        ///
        /// # Errors
        /// Returns unsupported operation unless the service exposes `GeoPackage` catalogs.
        fn inspect_geopackage(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _path: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<scriberig_app::GeoPackageCatalogPage, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "geo.catalog.inspect".into(),
            })
        }

        /// Reads one exact-snapshot QGIS project and its resolved layer references.
        ///
        /// # Parameters
        /// - `_context`: Trusted request context with exact snapshot and grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_path`: Exact project-relative QGS/QGZ path.
        ///
        /// # Returns
        /// Provider-neutral QGIS project and reference semantics.
        ///
        /// # Errors
        /// Returns unsupported operation unless QGIS project inspection is available.
        fn inspect_qgis_project(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _path: &str,
        ) -> Result<scriberig_app::QgisProjectResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "qgis.project.inspect".into(),
            })
        }
    };
}

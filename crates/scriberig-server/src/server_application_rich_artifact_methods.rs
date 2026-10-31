//! Adapts rich artifact reads and candidate edits to the shared application facade.

/// Supplies authorized rich artifact methods to the server application adapter.
macro_rules! server_application_rich_artifact_methods {
    () => {
        /// Lists project artifacts through the authorized shared application facade.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context.
        /// - `project`: Serialized stable project identity.
        /// - `offset`: Zero-based page offset.
        /// - `limit`: Maximum number of summaries.
        ///
        /// A bounded artifact page.
        ///
        /// # Returns
        /// The artifact manifest page.
        ///
        /// # Errors
        /// Returns an invalid-request error for malformed identities and the shared
        /// application error for inspection failures.
        fn inspect_artifacts(
            &self,
            context: &ApplicationContext,
            project: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::ArtifactManifestPage, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "artifact.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_artifacts(
                self, context, project, offset, limit,
            )
        }

        /// Reads a bounded artifact-member page through the authorized application facade.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context.
        /// - `project`: Serialized stable project identity.
        /// - `artifact`: Serialized stable artifact identity.
        /// - `offset`: Zero-based member offset.
        /// - `limit`: Maximum number of members.
        ///
        /// # Returns
        /// Exact physical and container-part evidence with continuation metadata.
        ///
        /// # Errors
        /// Returns invalid-identity or shared application errors.
        fn inspect_artifact_members(
            &self,
            context: &ApplicationContext,
            project: &str,
            artifact: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::ArtifactMembers, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "artifact.list_members".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let artifact = artifact
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "artifact.list_members".into(),
                    message: format!("invalid artifact identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_artifact_members(
                self, context, project, artifact, offset, limit,
            )
        }

        /// Reads a bounded worksheet cell page through the trusted application boundary.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context containing snapshot and grants.
        /// - `project`: Serialized stable project identity.
        /// - `worksheet`: Serialized stable worksheet identity.
        /// - `range`: Optional inclusive rectangular coordinate filter.
        /// - `offset`: Zero-based page offset.
        /// - `limit`: Maximum returned cell count.
        ///
        /// A snapshot-bound sparse cell page.
        ///
        /// # Returns
        /// The requested bounded cell page.
        ///
        /// # Errors
        /// Returns malformed-identity errors or shared application failures.
        fn read_spreadsheet_cells(
            &self,
            context: &ApplicationContext,
            project: &str,
            worksheet: &str,
            range: Option<scriberig_protocol::SpreadsheetRange>,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::SpreadsheetCells, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.cells.read".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let worksheet =
                worksheet
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "spreadsheet.cells.read".into(),
                        message: format!("invalid worksheet identity: {error}"),
                    })?;
            let range = range.map(|range| scriberig_domain_spreadsheet::CellRange {
                first_row: range.first_row,
                first_column: range.first_column,
                last_row: range.last_row,
                last_column: range.last_column,
            });
            scriberig_app::ApplicationService::read_spreadsheet_cells(
                self,
                context,
                project,
                worksheet,
                range.as_ref(),
                offset,
                limit,
            )
        }

        /// Reads one CAD drawing summary through the shared application Inspect operation.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context with an exact snapshot and grants.
        /// - `project`: Serialized stable project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `drawing`: Serialized semantic drawing identity.
        ///
        /// A typed CAD drawing summary and its layers.
        ///
        /// # Returns
        /// The exact-snapshot drawing view.
        ///
        /// # Errors
        /// Returns malformed-identity errors or shared application failures.
        fn inspect_cad_drawing(
            &self,
            context: &ApplicationContext,
            project: &str,
            snapshot: &str,
            drawing: &str,
        ) -> Result<scriberig_project::CadDrawingView, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.drawing.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let snapshot = snapshot
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.drawing.inspect".into(),
                    message: format!("invalid snapshot identity: {error}"),
                })?;
            let drawing = drawing
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.drawing.inspect".into(),
                    message: format!("invalid drawing identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_cad_drawing(
                self, context, project, snapshot, drawing,
            )
        }

        /// Reads one bounded CAD entity page through the shared application Inspect operation.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted request context with exact snapshot and grants.
        /// - `project`: Serialized stable project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `drawing`: Serialized semantic drawing identity.
        /// - `offset`: Zero-based entity offset.
        /// - `limit`: Bounded maximum entity count.
        ///
        /// A typed semantic CAD entity page.
        ///
        /// # Returns
        /// The requested page of drawing entities.
        ///
        /// # Errors
        /// Returns malformed-identity errors or shared application failures.
        fn read_cad_entities(
            &self,
            context: &ApplicationContext,
            project: &str,
            snapshot: &str,
            drawing: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_project::CadEntityPage, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.entities.read".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let snapshot = snapshot
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.entities.read".into(),
                    message: format!("invalid snapshot identity: {error}"),
                })?;
            let drawing = drawing
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.entities.read".into(),
                    message: format!("invalid drawing identity: {error}"),
                })?;
            scriberig_app::ApplicationService::read_cad_entities(
                self, context, project, snapshot, drawing, offset, limit,
            )
        }

        /// Finds authored CAD annotation text through the shared application operation.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot Inspect context.
        /// - `request`: Exact-snapshot drawing, query, and bounded page parameters.
        ///
        /// # Returns
        /// Bounded text matches with stable semantic entity identities.
        ///
        /// # Errors
        /// Returns malformed identity errors or shared application failures.
        fn find_cad_text(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::CadTextSearchRequest,
        ) -> Result<scriberig_project::CadTextSearch, ApplicationError> {
            scriberig_app::ApplicationService::find_cad_text(self, context, request)
        }

        /// Reads bounded CAD entities intersecting one drawing-space envelope.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot Inspect context.
        /// - `request`: Validated semantic CAD bounds query.
        ///
        /// # Returns
        /// Bounded semantic matches and extent completeness metadata.
        ///
        /// # Errors
        /// Returns shared application authorization or query errors.
        fn read_cad_bounds_query(
            &self,
            context: &ApplicationContext,
            request: scriberig_project::CadBoundsQueryRequest,
        ) -> Result<scriberig_project::CadBoundsEntityPage, ApplicationError> {
            scriberig_app::ApplicationService::read_cad_entities_intersecting_bounds(
                self, context, request,
            )
        }

        /// Creates one exact-base CAD line candidate through the shared application facade.
        ///
        /// - `self`: Shared application service.
        /// - `context`: Trusted application request context.
        /// - `project`: Serialized stable project identity.
        /// - `entity`: Serialized semantic CAD entity identity.
        /// - `start`: Exact decimal start coordinate.
        /// - `end`: Exact decimal end coordinate.
        ///
        /// Durable candidate metadata for review and validation.
        ///
        /// # Returns
        /// The candidate awaiting normal validation and promotion.
        ///
        /// # Errors
        /// Returns malformed-identity errors or shared application failures.
        fn propose_cad_line_edit(
            &self,
            context: &ApplicationContext,
            project: &str,
            entity: &str,
            start: scriberig_protocol::CadCoordinate,
            end: scriberig_protocol::CadCoordinate,
        ) -> Result<scriberig_app::CadLineCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.line.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let entity = entity
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.line.propose".into(),
                    message: format!("invalid CAD entity identity: {error}"),
                })?;
            scriberig_app::ApplicationService::propose_cad_line_geometry(
                self,
                context,
                project,
                entity,
                &scriberig_domain_cad::Coordinate::new(start.x, start.y, start.z),
                &scriberig_domain_cad::Coordinate::new(end.x, end.y, end.z),
            )
        }

        /// Creates one exact-base CAD layer candidate through the shared application facade.
        ///
        /// # Parameters
        /// - `context`: Trusted application request context.
        /// - `project`: Serialized stable project identity.
        /// - `entity`: Serialized semantic CAD entity identity.
        /// - `destination_layer`: Serialized semantic layer identity.
        ///
        /// # Returns
        /// Candidate metadata awaiting normal validation and promotion.
        ///
        /// # Errors
        /// Returns malformed-identity errors or shared application failures.
        fn propose_cad_layer_change(
            &self,
            context: &ApplicationContext,
            project: &str,
            entity: &str,
            destination_layer: &str,
        ) -> Result<scriberig_app::CadLayerCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.layer.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let entity = entity
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "cad.layer.propose".into(),
                    message: format!("invalid CAD entity identity: {error}"),
                })?;
            let destination_layer =
                destination_layer
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "cad.layer.propose".into(),
                        message: format!("invalid CAD layer identity: {error}"),
                    })?;
            scriberig_app::ApplicationService::propose_cad_layer_change(
                self,
                context,
                project,
                entity,
                destination_layer,
            )
        }
    };
}

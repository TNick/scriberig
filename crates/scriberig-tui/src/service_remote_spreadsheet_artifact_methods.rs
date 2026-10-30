//! Defines spreadsheet and artifact methods for the remote TUI service.

macro_rules! remote_spreadsheet_artifact_methods {
    () => {
        /// Proposes a spreadsheet edit through the typed remote candidate operation.
        ///
        /// # Parameters
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Exact project, snapshot, worksheet, coordinate, and authored value.
        ///
        /// # Returns
        /// Candidate identity and validation state for the shared review flow.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, response identity, or validation failures.
        fn propose_spreadsheet_edit(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellEditQuery,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            let response = self
                .client
                .propose_spreadsheet_cell_edit(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.worksheet.to_string(),
                    request.row,
                    request.column,
                    crate::view::spreadsheet::value_from_application(request.value.clone()),
                )
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.base_snapshot != request.snapshot.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote spreadsheet candidate identity does not match the request"
                        .to_owned(),
                });
            }
            let validation = match response.validation {
                scriberig_protocol::SpreadsheetCandidateValidation::Pending => {
                    scriberig_project::CandidateValidation::Pending
                }
                scriberig_protocol::SpreadsheetCandidateValidation::Passed => {
                    scriberig_project::CandidateValidation::Passed
                }
                scriberig_protocol::SpreadsheetCandidateValidation::Failed { diagnostics } => {
                    scriberig_project::CandidateValidation::Failed { diagnostics }
                }
            };
            Ok(scriberig_app::SpreadsheetEditCandidate {
                id: response.candidate_id,
                project: request.project,
                base_snapshot: request.snapshot,
                path: response.path,
                validation,
            })
        }

        /// Inspects one exact worksheet coordinate through the typed protocol client.
        ///
        /// # Parameters
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Exact project, snapshot, worksheet, row, and column.
        ///
        /// # Returns
        /// Typed coordinate details, including an explicit sparse blank result.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, or response failures.
        fn inspect_spreadsheet_cell(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellInspectResponse, ApplicationError> {
            self.client
                .inspect_spreadsheet_cell(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.worksheet.to_string(),
                    request.row,
                    request.column,
                )
                .map_err(remote_error)
        }

        /// Reads an exact-snapshot worksheet page through the typed protocol client.
        ///
        /// # Parameters
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
        ///
        /// # Returns
        /// Typed protocol cell page with fidelity and continuation metadata.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, or response failures.
        fn inspect_spreadsheet_cells(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellsQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellsResponse, ApplicationError> {
            self.client
                .read_spreadsheet_cells(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.worksheet.to_string(),
                    request
                        .range
                        .map(|value| scriberig_protocol::SpreadsheetRange {
                            first_row: value.first_row,
                            first_column: value.first_column,
                            last_row: value.last_row,
                            last_column: value.last_column,
                        }),
                    request.offset,
                    request.limit,
                )
                .map_err(remote_error)
        }

        /// Reads bounded artifact details through the remote typed protocol client.
        ///
        /// # Parameters
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `project`: Stable project identity.
        /// - `offset`: Zero-based artifact offset.
        /// - `limit`: Maximum number of artifact rows.
        ///
        /// # Returns
        /// Terminal-safe artifact metadata normalized from the protocol response.
        ///
        /// # Errors
        /// Returns a sanitized remote protocol or application failure.
        fn inspect_artifact_page(
            &self,
            _context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            offset: usize,
            limit: usize,
        ) -> Result<crate::view::artifacts::ArtifactPageModel, ApplicationError> {
            self.client
                .inspect_artifacts(project.to_string(), offset, limit)
                .map(crate::view::artifacts::ArtifactPageModel::from_protocol)
                .map_err(remote_error)
        }

        /// Reads a bounded exact artifact-member page through the remote client.
        ///
        /// # Parameters
        /// - `_context`: Local context; remote authorization is transport-owned.
        /// - `project`: Stable project identity.
        /// - `artifact`: Stable semantic artifact identity.
        /// - `offset`: Zero-based member offset.
        /// - `limit`: Maximum number of member descriptors.
        ///
        /// # Returns
        /// Terminal-safe member evidence normalized from the protocol response.
        ///
        /// # Errors
        /// Returns a sanitized remote protocol or application failure.
        fn inspect_artifact_members(
            &self,
            _context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            artifact: scriberig_core::EntityId,
            offset: usize,
            limit: usize,
        ) -> Result<crate::view::ArtifactMembersModel, ApplicationError> {
            self.client
                .inspect_artifact_members(project.to_string(), artifact.to_string(), offset, limit)
                .map(crate::view::ArtifactMembersModel::from_protocol)
                .map_err(remote_error)
        }
    };
}

//! Defines geopackage and semantic document methods for the remote TUI service.

macro_rules! remote_geopackage_document_methods {
    () => {
        /// Reads a bounded `GeoPackage` catalog through the typed remote client.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `project`: Stable owning project identity.
        /// - `path`: Exact project-relative artifact path.
        /// - `snapshot`: Exact immutable source snapshot.
        /// - `offset`: Zero-based layer offset.
        ///
        /// # Returns
        /// Typed dataset and layer metadata with exact request scope.
        ///
        /// # Errors
        /// Returns transport errors or a response whose project/snapshot differs.
        fn inspect_geopackage(
            &self,
            _context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            path: &str,
            snapshot: SnapshotId,
            offset: usize,
        ) -> Result<scriberig_protocol::GeoCatalogResponse, ApplicationError> {
            let response = self
                .client
                .inspect_geopackage(
                    project.to_string(),
                    path.to_owned(),
                    snapshot.to_string(),
                    offset,
                    40,
                )
                .map_err(remote_error)?;
            if response.project != project.to_string() || response.snapshot != snapshot.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote GeoPackage catalog does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Reads a bounded `GeoPackage` feature page through the typed remote client.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `project`: Stable owning project identity.
        /// - `path`: Exact project-relative artifact path.
        /// - `snapshot`: Exact immutable source snapshot.
        /// - `layer`: Stable semantic layer identity.
        /// - `offset`: Zero-based feature offset.
        ///
        /// # Returns
        /// Typed feature page with exact request scope.
        ///
        /// # Errors
        /// Returns transport errors or a response whose project/snapshot/layer differs.
        fn read_geopackage_features(
            &self,
            _context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            path: &str,
            snapshot: SnapshotId,
            layer: EntityId,
            offset: usize,
        ) -> Result<scriberig_protocol::GeoFeaturesResponse, ApplicationError> {
            let response = self
                .client
                .geo_features(
                    project.to_string(),
                    path.to_owned(),
                    snapshot.to_string(),
                    layer.to_string(),
                    offset,
                    40,
                )
                .map_err(remote_error)?;
            if response.project != project.to_string()
                || response.snapshot != snapshot.to_string()
                || response.layer != layer.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote GeoPackage feature page does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Proposes a typed geospatial attribute edit through the protocol client.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `edit`: Exact project, snapshot, feature, field, and replacement value.
        ///
        /// # Returns
        /// Durable candidate metadata for common validation, review, and apply.
        ///
        /// # Errors
        /// Returns transport errors or a candidate whose exact request scope differs.
        fn propose_geo_attribute_edit(
            &self,
            _context: &ApplicationContext,
            edit: &scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, ApplicationError> {
            let response = self
                .client
                .propose_geo_attribute_edit(scriberig_protocol::GeoAttributeEditRequest {
                    project: edit.project.to_string(),
                    path: edit.path.clone(),
                    snapshot: edit.base_snapshot.to_string(),
                    layer: edit.layer.to_string(),
                    feature: edit.feature.to_string(),
                    field: edit.field.clone(),
                    value: crate::view::geo::value_to_protocol(edit.value.clone()),
                })
                .map_err(remote_error)?;
            if response.project != edit.project.to_string()
                || response.base_snapshot != edit.base_snapshot.to_string()
                || response.layer != edit.layer.to_string()
                || response.feature != edit.feature.to_string()
                || response.field != edit.field
                || response.path != edit.path
            {
                return Err(ApplicationError::Internal {
                    message: "remote geospatial candidate does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(scriberig_app::GeoAttributeCandidate {
                id: response.candidate,
                project: edit.project,
                base_snapshot: edit.base_snapshot,
                feature: edit.feature,
                layer: edit.layer,
                field: edit.field.clone(),
                path: response.path,
                validation: scriberig_project::CandidateValidation::Pending,
            })
        }

        /// Proposes typed `GeoPackage` geometry through the remote candidate API.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `request`: Exact project, source, snapshot, target, and geometry.
        ///
        /// # Returns
        /// Candidate metadata preserving exact target identities.
        ///
        /// # Errors
        /// Returns transport errors or a candidate whose exact request scope differs.
        fn propose_geo_geometry_edit(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::GeoGeometryEditRequest,
        ) -> Result<scriberig_app::GeoGeometryCandidate, ApplicationError> {
            let response = self
                .client
                .propose_geo_geometry_edit(
                    request.project.clone(),
                    request.path.clone(),
                    request.snapshot.clone(),
                    request.layer.clone(),
                    request.feature.clone(),
                    request.geometry.clone(),
                )
                .map_err(remote_error)?;
            if response.project != request.project
                || response.base_snapshot != request.snapshot
                || response.layer != request.layer
                || response.feature != request.feature
                || response.path != request.path
            {
                return Err(ApplicationError::Internal {
                    message:
                        "remote geospatial geometry candidate does not match the exact request"
                            .to_owned(),
                });
            }
            Ok(scriberig_app::GeoGeometryCandidate {
                id: response.candidate,
                project: request
                    .project
                    .parse()
                    .map_err(|_| ApplicationError::Internal {
                        message: "remote geometry candidate has an invalid project identity"
                            .to_owned(),
                    })?,
                base_snapshot: request.snapshot.parse().map_err(|_| {
                    ApplicationError::Internal {
                        message: "remote geometry candidate has an invalid snapshot identity"
                            .to_owned(),
                    }
                })?,
                feature: request
                    .feature
                    .parse()
                    .map_err(|_| ApplicationError::Internal {
                        message: "remote geometry candidate has an invalid feature identity"
                            .to_owned(),
                    })?,
                layer: request
                    .layer
                    .parse()
                    .map_err(|_| ApplicationError::Internal {
                        message: "remote geometry candidate has an invalid layer identity"
                            .to_owned(),
                    })?,
                path: response.path,
                validation: scriberig_project::CandidateValidation::Pending,
            })
        }

        /// Reads a bounded document tree from the exact remote snapshot.
        ///
        /// # Parameters
        /// - `_context`: Authorization is resolved by the remote transport host.
        /// - `request`: Stable project, snapshot, and document identity.
        ///
        /// # Returns
        /// Protocol-shaped semantic sections, paragraphs, and tables.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, or response-identity errors.
        fn inspect_document(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::DocumentTreeQuery,
        ) -> Result<scriberig_protocol::DocumentInspectResponse, ApplicationError> {
            let response = self
                .client
                .inspect_document(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.document.to_string(),
                )
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.snapshot != request.snapshot.to_string()
                || response.document != request.document.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote document response does not match the exact request".to_owned(),
                });
            }
            Ok(response)
        }

        /// Proposes a paragraph edit through the typed remote candidate API.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `request`: Project, snapshot, semantic paragraph, and replacement.
        ///
        /// # Returns
        /// Durable candidate metadata for the shared Changes review.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, response identity, or candidate errors.
        fn propose_document_paragraph_edit(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::DocumentParagraphEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            let response = self
                .client
                .propose_document_paragraph_edit(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.paragraph.to_string(),
                    request.replacement.clone(),
                )
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.base_snapshot != request.snapshot.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote document candidate does not match the exact request"
                        .to_owned(),
                });
            }
            let validation = match response.validation {
                scriberig_protocol::DocumentCandidateValidation::Pending => {
                    scriberig_project::CandidateValidation::Pending
                }
                scriberig_protocol::DocumentCandidateValidation::Passed => {
                    scriberig_project::CandidateValidation::Passed
                }
                scriberig_protocol::DocumentCandidateValidation::Failed { diagnostics } => {
                    scriberig_project::CandidateValidation::Failed { diagnostics }
                }
            };
            Ok(scriberig_app::DocumentParagraphCandidate {
                id: response.candidate_id,
                project: request.project,
                base_snapshot: request.snapshot,
                path: response.path,
                validation,
            })
        }

        /// Proposes a table-cell edit through the typed remote candidate API.
        ///
        /// # Parameters
        /// - `_context`: Remote authorization is transport-owned.
        /// - `request`: Project, snapshot, semantic cell, and replacement.
        ///
        /// # Returns
        /// Durable candidate metadata for the shared Changes review.
        ///
        /// # Errors
        /// Returns sanitized transport, protocol, response identity, or candidate errors.
        fn propose_document_table_cell_edit(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::DocumentTableCellEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            let response = self
                .client
                .propose_document_table_cell_edit(
                    request.project.to_string(),
                    request.snapshot.to_string(),
                    request.cell.to_string(),
                    request.replacement.clone(),
                )
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.base_snapshot != request.snapshot.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote table-cell candidate does not match the exact request"
                        .to_owned(),
                });
            }
            let validation = match response.validation {
                scriberig_protocol::DocumentCandidateValidation::Pending => {
                    scriberig_project::CandidateValidation::Pending
                }
                scriberig_protocol::DocumentCandidateValidation::Passed => {
                    scriberig_project::CandidateValidation::Passed
                }
                scriberig_protocol::DocumentCandidateValidation::Failed { diagnostics } => {
                    scriberig_project::CandidateValidation::Failed { diagnostics }
                }
            };
            Ok(scriberig_app::DocumentParagraphCandidate {
                id: response.candidate_id,
                project: request.project,
                base_snapshot: request.snapshot,
                path: response.path,
                validation,
            })
        }
    };
}

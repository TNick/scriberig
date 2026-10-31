//! Provides cad qgis methods for the application operation adapter.

macro_rules! operation_cad_qgis_methods {
    () => {
        /// Reads an exact-snapshot CAD drawing through the authorized application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `request`: Exact project, snapshot, and drawing identity.
        ///
        /// # Returns
        /// Typed drawing metadata with semantic layer summaries.
        ///
        /// # Errors
        /// Returns authorization, identity parsing, snapshot, or project read errors.
        fn inspect_cad_drawing(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::CadDrawingInspectRequest,
        ) -> Result<scriberig_protocol::CadDrawingInspectResponse, ApplicationError> {
            let project = request
                .project
                .parse()
                .map_err(|_| invalid_cad_request("project"))?;
            let snapshot = request
                .snapshot
                .parse()
                .map_err(|_| invalid_cad_request("snapshot"))?;
            let drawing = request
                .drawing
                .parse()
                .map_err(|_| invalid_cad_request("drawing"))?;
            self.service
                .inspect_cad_drawing(
                    &context.clone().with_snapshot(snapshot),
                    project,
                    snapshot,
                    drawing,
                )
                .map(crate::view::cad::drawing_from_application)
        }

        /// Reads one bounded CAD entity page through the authorized application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `request`: Exact drawing identity and page bounds.
        ///
        /// # Returns
        /// Typed semantic entity summaries with continuation metadata.
        ///
        /// # Errors
        /// Returns authorization, identity parsing, snapshot, or project read errors.
        fn inspect_cad_entities(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::CadEntitiesRequest,
        ) -> Result<scriberig_protocol::CadEntitiesResponse, ApplicationError> {
            let project = request
                .project
                .parse()
                .map_err(|_| invalid_cad_request("project"))?;
            let snapshot = request
                .snapshot
                .parse()
                .map_err(|_| invalid_cad_request("snapshot"))?;
            let drawing = request
                .drawing
                .parse()
                .map_err(|_| invalid_cad_request("drawing"))?;
            self.service
                .read_cad_entities(
                    &context.clone().with_snapshot(snapshot),
                    project,
                    snapshot,
                    drawing,
                    request.offset,
                    request.limit,
                )
                .map(crate::view::cad::entities_from_application)
        }

        /// Proposes an exact-base CAD layer change through the shared application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `request`: Exact project, snapshot, entity, and destination layer.
        ///
        /// # Returns
        /// Reviewable candidate metadata for common Changes review.
        ///
        /// # Errors
        /// Returns authorization, identity parsing, stale-base, or candidate errors.
        fn propose_cad_layer_change(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::CadLayerChangeRequest,
        ) -> Result<scriberig_protocol::CadLayerChangeResponse, ApplicationError> {
            let project = request
                .project
                .parse()
                .map_err(|_| invalid_cad_request("project"))?;
            let snapshot = request
                .snapshot
                .parse()
                .map_err(|_| invalid_cad_request("snapshot"))?;
            let entity = request
                .entity
                .parse()
                .map_err(|_| invalid_cad_request("entity"))?;
            let destination = request
                .destination_layer
                .parse()
                .map_err(|_| invalid_cad_request("destination layer"))?;
            self.service
                .propose_cad_layer_change(
                    &context.clone().with_snapshot(snapshot),
                    project,
                    entity,
                    destination,
                )
                .map(|candidate| crate::view::cad::layer_candidate_from_project(&candidate))
        }

        /// Proposes an exact-base QGIS layer rename through the shared application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `request`: Exact project, snapshot, reference, and new display name.
        ///
        /// # Returns
        /// Candidate handle for common review and apply.
        ///
        /// # Errors
        /// Returns authorization, identity parsing, stale-base, or candidate errors.
        fn propose_qgis_layer_rename(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::QgisLayerRenameRequest,
        ) -> Result<scriberig_protocol::QgisLayerRenameResponse, ApplicationError> {
            let project =
                request
                    .project
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "qgis.layer.rename".to_owned(),
                        message: "project identity is invalid".to_owned(),
                    })?;
            let snapshot =
                request
                    .snapshot
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "qgis.layer.rename".to_owned(),
                        message: "snapshot identity is invalid".to_owned(),
                    })?;
            let reference =
                request
                    .reference
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "qgis.layer.rename".to_owned(),
                        message: "layer reference identity is invalid".to_owned(),
                    })?;
            let candidate = self.service.propose_qgis_layer_rename(
                &context.clone().with_snapshot(snapshot),
                project,
                reference,
                &request.new_name,
            )?;
            Ok(scriberig_protocol::QgisLayerRenameResponse {
                candidate: candidate.id,
                project: candidate.project.to_string(),
                base_snapshot: candidate.base_snapshot.to_string(),
                reference: candidate.reference.to_string(),
                path: candidate.path,
            })
        }

        /// Proposes a QGIS layer order through the local shared application API.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, path, and complete layer order.
        ///
        /// # Returns
        /// Candidate metadata for the shared review/apply lifecycle.
        ///
        /// # Errors
        /// Returns invalid identities, authorization, stale-base, or candidate errors.
        fn propose_qgis_layer_order(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::QgisLayerOrderRequest,
        ) -> Result<scriberig_protocol::QgisLayerOrderResponse, ApplicationError> {
            let project =
                request
                    .project
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "qgis.layer_order.reorder".to_owned(),
                        message: "project identity is invalid".to_owned(),
                    })?;
            let snapshot =
                request
                    .snapshot
                    .parse()
                    .map_err(|_| ApplicationError::InvalidRequest {
                        operation: "qgis.layer_order.reorder".to_owned(),
                        message: "snapshot identity is invalid".to_owned(),
                    })?;
            let ordered_references = request
                .ordered_references
                .iter()
                .map(|reference| {
                    reference
                        .parse()
                        .map_err(|_| ApplicationError::InvalidRequest {
                            operation: "qgis.layer_order.reorder".to_owned(),
                            message: "layer reference identity is invalid".to_owned(),
                        })
                })
                .collect::<Result<Vec<scriberig_core::EntityId>, _>>()?;
            let candidate = self.service.propose_qgis_layer_order(
                &context.clone().with_snapshot(snapshot),
                project,
                &request.path,
                &ordered_references,
            )?;
            Ok(scriberig_protocol::QgisLayerOrderResponse {
                candidate: candidate.id,
                project: candidate.project.to_string(),
                base_snapshot: candidate.base_snapshot.to_string(),
                path: candidate.path,
            })
        }

        /// Sets or clears the trusted caller's personal Rust source style.
        ///
        /// # Parameters
        /// - `context`: Trusted caller context supplying the personal principal.
        /// - `request`: User-scoped style write or clear.
        ///
        /// # Returns
        /// The authoritative style value and setting revision.
        ///
        /// # Errors
        /// Returns invalid-kind, principal, validation, compare-and-swap, or storage errors.
        fn set_projection_style_preference(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleSettingRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleSettingResponse, ApplicationError> {
            if request.projection_kind != "rust"
                || request.scope != scriberig_protocol::ProjectionStyleScope::User
            {
                return Err(ApplicationError::InvalidRequest {
                    operation: "projection style preference".to_owned(),
                    message: "the TUI accepts only personal Rust source styles".to_owned(),
                });
            }
            let kind =
                sr_projection::ProjectionKind::new(&request.projection_kind).map_err(|error| {
                    ApplicationError::InvalidRequest {
                        operation: "projection style preference".to_owned(),
                        message: error.to_string(),
                    }
                })?;
            let stored =
                scriberig_app::ProjectionStyleBackend::set_projection_style_user_preference(
                    &self.service,
                    context,
                    &scriberig_app::ProjectionStyleSettingRequest {
                        projection_kind: kind,
                        scope_id: String::new(),
                        expected_revision: request.expected_revision,
                        style: request.style.clone().map(sr_projection::StyleValue::new),
                    },
                )?;
            Ok(scriberig_protocol::ProjectionStyleSettingResponse {
                style: stored
                    .as_ref()
                    .map(|setting| setting.value.as_bytes().to_vec()),
                revision: stored.map(|setting| setting.revision),
            })
        }

        /// Resolves effective Rust style evidence through the shared application resolver.
        ///
        /// # Parameters
        /// - `context`: Trusted caller context supplying personal style identity.
        /// - `request`: Exact style scopes, view, and purpose.
        ///
        /// # Returns
        /// Canonical style bytes with provenance and fidelity evidence.
        ///
        /// # Errors
        /// Returns invalid request, schema, configuration, or storage errors.
        fn resolve_projection_style(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::ProjectionStyleResolveRequest,
        ) -> Result<scriberig_protocol::ProjectionStyleResolveResponse, ApplicationError> {
            if request.projection_kind != "rust"
                || request.purpose != scriberig_protocol::ProjectionStylePurpose::InteractiveView
            {
                return Err(ApplicationError::InvalidRequest {
                    operation: "projection style resolution".to_owned(),
                    message: "the TUI resolves only interactive Rust source styles".to_owned(),
                });
            }
            let kind =
                sr_projection::ProjectionKind::new(&request.projection_kind).map_err(|error| {
                    ApplicationError::InvalidRequest {
                        operation: "projection style resolution".to_owned(),
                        message: error.to_string(),
                    }
                })?;
            let mut app_request = scriberig_app::StyleResolutionRequest::new(
                kind,
                &request.project_id,
                &request.workspace_id,
                sr_projection::ProjectionPurpose::InteractiveView,
            );
            app_request.principal = context
                .principal
                .as_ref()
                .map(|principal| principal.id.clone());
            app_request.view_id.clone_from(&request.view_id);
            let effective = scriberig_app::ProjectionStyleBackend::projection_style_effective(
                &self.service,
                context,
                &app_request,
            )?;
            Ok(crate::view::projection_style::from_application(effective))
        }

        /// Reads a bounded `GeoPackage` catalog through the authorized application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and application context.
        /// - `project`: Stable owning project identity.
        /// - `path`: Exact project-relative artifact path.
        /// - `snapshot`: Exact immutable source snapshot.
        /// - `offset`: Zero-based layer offset.
        ///
        /// # Returns
        /// Protocol-shaped dataset and layer metadata.
        ///
        /// # Errors
        /// Returns authorization, source, snapshot, or normalization errors.
        fn inspect_geopackage(
            &self,
            context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            path: &str,
            snapshot: SnapshotId,
            offset: usize,
        ) -> Result<scriberig_protocol::GeoCatalogResponse, ApplicationError> {
            self.service
                .inspect_geopackage(
                    &context.clone().with_snapshot(snapshot),
                    project,
                    path,
                    offset,
                    40,
                )
                .map(crate::view::geo::catalog_from_application)
        }
    };
}

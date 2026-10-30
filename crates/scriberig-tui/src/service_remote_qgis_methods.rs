//! Defines qgis and projection-style methods for the remote TUI service.

macro_rules! remote_qgis_methods {
    () => {
        /// Reads a QGIS project through the remote protocol client.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact project, path, and snapshot scope.
        ///
        /// # Returns
        /// Provider-neutral QGIS references and resolved source paths.
        ///
        /// # Errors
        /// Returns transport errors or a response outside the requested scope.
        fn inspect_qgis_project(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_app::QgisProjectQuery,
        ) -> Result<scriberig_protocol::QgisProjectResponse, ApplicationError> {
            let response = self
                .client
                .inspect_qgis_project(scriberig_protocol::QgisProjectRequest {
                    project: request.project.to_string(),
                    path: request.path.clone(),
                    snapshot: request.snapshot.to_string(),
                })
                .map_err(remote_error)?;
            if response.project != request.project.to_string()
                || response.path != request.path
                || response.snapshot != request.snapshot.to_string()
            {
                return Err(ApplicationError::Internal {
                    message: "remote QGIS project does not match the exact request".to_owned(),
                });
            }
            Ok(response)
        }

        /// Proposes a QGIS layer rename through the remote candidate API.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact project, snapshot, reference, and new display name.
        ///
        /// # Returns
        /// Candidate handle for common review and apply.
        ///
        /// # Errors
        /// Returns transport errors or a candidate outside the exact request scope.
        fn propose_qgis_layer_rename(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::QgisLayerRenameRequest,
        ) -> Result<scriberig_protocol::QgisLayerRenameResponse, ApplicationError> {
            let response = self
                .client
                .propose_qgis_layer_rename(request.clone())
                .map_err(remote_error)?;
            if response.project != request.project
                || response.base_snapshot != request.snapshot
                || response.reference != request.reference
            {
                return Err(ApplicationError::Internal {
                    message: "remote QGIS rename candidate does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Proposes a QGIS layer order through the remote candidate API.
        ///
        /// # Parameters
        /// - `_context`: Authorization is applied by the remote host.
        /// - `request`: Exact project, snapshot, path, and complete layer order.
        ///
        /// # Returns
        /// Candidate handle for common review and apply.
        ///
        /// # Errors
        /// Returns transport errors or a candidate outside the exact request scope.
        fn propose_qgis_layer_order(
            &self,
            _context: &ApplicationContext,
            request: &scriberig_protocol::QgisLayerOrderRequest,
        ) -> Result<scriberig_protocol::QgisLayerOrderResponse, ApplicationError> {
            let response = self
                .client
                .propose_qgis_layer_order(request.clone())
                .map_err(remote_error)?;
            if response.project != request.project
                || response.base_snapshot != request.snapshot
                || response.path != request.path
            {
                return Err(ApplicationError::Internal {
                    message: "remote QGIS order candidate does not match the exact request"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Sets or clears the trusted caller's personal Rust source style remotely.
        ///
        /// # Parameters
        /// - `_context`: Personal identity and authorization are applied by the remote host.
        /// - `request`: User-scoped style write or clear.
        ///
        /// # Returns
        /// The authoritative style value and setting revision.
        ///
        /// # Errors
        /// Returns transport errors or a response with inconsistent set/clear state.
        fn set_projection_style_preference(
            &self,
            _context: &ApplicationContext,
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
            let response = self
                .client
                .set_projection_style(request.clone())
                .map_err(remote_error)?;
            if response.style.is_some() != request.style.is_some()
                || response.revision.is_some() != request.style.is_some()
            {
                return Err(ApplicationError::Internal {
                    message: "remote projection style response has inconsistent setting state"
                        .to_owned(),
                });
            }
            Ok(response)
        }

        /// Resolves exact interactive Rust style evidence through the remote host.
        ///
        /// # Parameters
        /// - `_context`: Personal identity and authorization are applied by the remote host.
        /// - `request`: Exact project, workspace, view, and purpose.
        ///
        /// # Returns
        /// Canonical style bytes with provenance and fidelity evidence.
        ///
        /// # Errors
        /// Returns transport errors or a response for a different resolution purpose.
        fn resolve_projection_style(
            &self,
            _context: &ApplicationContext,
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
            let response = self
                .client
                .resolve_projection_style(request.clone())
                .map_err(remote_error)?;
            if response.purpose != request.purpose {
                return Err(ApplicationError::Internal {
                    message: "remote projection style response has a mismatched purpose".to_owned(),
                });
            }
            Ok(response)
        }
    };
}

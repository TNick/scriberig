//! Forwards borrowed-service rich-artifact candidate operations.

macro_rules! shared_borrowed_artifact_candidates {
    () => {
        /// Delegates QGIS layer-reference rename proposals through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and Propose grants.
        /// - `project`: Serialized owning project identity.
        /// - `reference`: Serialized semantic layer-reference identity.
        /// - `new_name`: New authored display name.
        ///
        /// # Returns
        /// Candidate metadata for shared validation, review, and apply.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate failure.
        fn propose_qgis_layer_rename(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            reference: &str,
            new_name: &str,
        ) -> Result<scriberig_app::QgisLayerRenameCandidate, scriberig_app::ApplicationError> {
            (**self).propose_qgis_layer_rename(context, project, reference, new_name)
        }

        /// Delegates QGIS project layer-order candidates through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and Propose grants.
        /// - `project`: Serialized project identity.
        /// - `path`: Exact project-relative QGS/QGZ path.
        /// - `ordered_references`: Complete semantic layer-reference permutation.
        ///
        /// # Returns
        /// Durable candidate metadata.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate failure.
        fn propose_qgis_layer_order(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            path: &str,
            ordered_references: &[String],
        ) -> Result<scriberig_app::QgisLayerOrderCandidate, scriberig_app::ApplicationError> {
            (**self).propose_qgis_layer_order(context, project, path, ordered_references)
        }

        /// Delegates exact-base `GeoPackage` geometry candidates through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and Propose grants.
        /// - `project`: Serialized project identity.
        /// - `path`: Project-relative `GeoPackage` source path.
        /// - `layer`: Serialized semantic layer identity.
        /// - `feature`: Serialized semantic feature identity.
        /// - `geometry`: Typed replacement geometry in layer CRS.
        ///
        /// # Returns
        /// Durable candidate metadata.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_geo_feature_geometry(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            path: &str,
            layer: &str,
            feature: &str,
            geometry: &scriberig_domain_geo::Geometry,
        ) -> Result<scriberig_app::GeoGeometryCandidate, scriberig_app::ApplicationError> {
            (**self).propose_geo_feature_geometry(context, project, path, layer, feature, geometry)
        }

        /// Delegates an exact-base feature attribute candidate through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context and Propose grants.
        /// - `edit`: Exact project, source, feature, field, and typed value.
        ///
        /// # Returns
        /// Durable candidate metadata.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_geo_feature_attribute(
            &self,
            context: &scriberig_app::ApplicationContext,
            edit: scriberig_project::GeoPackageFeatureAttributeEdit,
        ) -> Result<scriberig_app::GeoAttributeCandidate, scriberig_app::ApplicationError> {
            (**self).propose_geo_feature_attribute(context, edit)
        }

        /// Delegates a semantic document paragraph insertion through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `section`: Serialized semantic section identity.
        /// - `text`: New authored paragraph text.
        ///
        /// # Returns
        /// Candidate metadata without DOCX bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_document_paragraph_insert(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            section: &str,
            text: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            (**self).propose_document_paragraph_insert(context, project, section, text)
        }

        /// Delegates a semantic document paragraph edit through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `paragraph`: Serialized semantic paragraph identity.
        /// - `replacement`: New authored text.
        ///
        /// # Returns
        /// Candidate metadata without DOCX bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_document_paragraph_edit(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            paragraph: &str,
            replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            (**self).propose_document_paragraph_edit(context, project, paragraph, replacement)
        }

        /// Delegates a semantic document table-cell edit through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `cell`: Serialized semantic table-cell identity.
        /// - `replacement`: New authored cell text.
        ///
        /// # Returns
        /// Candidate metadata without DOCX bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_document_table_cell_edit(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            cell: &str,
            replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            (**self).propose_document_table_cell_edit(context, project, cell, replacement)
        }
    };
}

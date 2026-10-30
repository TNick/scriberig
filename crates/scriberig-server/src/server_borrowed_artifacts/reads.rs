//! Forwards borrowed-service semantic reads for rich artifacts.

macro_rules! shared_borrowed_artifact_reads {
    () => {
        shared_borrowed_spreadsheet_methods!();
        /// Delegates bounded document paragraph search through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `request`: Typed project, snapshot, document, query, and page selection.
        ///
        /// # Returns
        /// Bounded ordered paragraph matches.
        ///
        /// # Errors
        /// Returns the delegated authorization or semantic search error.
        fn find_document_paragraphs(
            &self,
            context: &scriberig_app::ApplicationContext,
            request: &scriberig_app::DocumentParagraphSearchRequest,
        ) -> Result<scriberig_app::DocumentParagraphSearch, scriberig_app::ApplicationError> {
            (**self).find_document_paragraphs(context, request)
        }

        /// Delegates persisted artifact inspection through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context.
        /// - `project`: Serialized project identity.
        /// - `offset`: Zero-based artifact offset.
        /// - `limit`: Maximum artifact page size.
        ///
        /// # Returns
        /// Bounded artifact manifest page.
        ///
        /// # Errors
        /// Returns the delegated service error.
        fn inspect_artifacts(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::ArtifactManifestPage, scriberig_app::ApplicationError> {
            (**self).inspect_artifacts(context, project, offset, limit)
        }

        /// Delegates bounded exact artifact-member inspection through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context.
        /// - `project`: Serialized project identity.
        /// - `artifact`: Serialized artifact identity.
        /// - `offset`: Zero-based member offset.
        /// - `limit`: Maximum member page size.
        ///
        /// # Returns
        /// Bounded artifact-member evidence.
        ///
        /// # Errors
        /// Returns the delegated service error.
        fn inspect_artifact_members(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            artifact: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::ArtifactMembers, scriberig_app::ApplicationError> {
            (**self).inspect_artifact_members(context, project, artifact, offset, limit)
        }

        /// Delegates a snapshot-bound `GeoPackage` feature read through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `path`: Project-relative `GeoPackage` source path.
        /// - `layer`: Serialized semantic layer identity.
        /// - `offset`: Zero-based feature offset.
        /// - `limit`: Maximum feature page size.
        ///
        /// Bounded provider-neutral feature page.
        ///
        /// # Returns
        /// The delegated feature page.
        ///
        /// # Errors
        /// Returns the delegated service error.
        fn read_geopackage_features(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            path: &str,
            layer: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::GeoFeaturePage, scriberig_app::ApplicationError> {
            (**self).read_geopackage_features(context, project, path, layer, offset, limit)
        }

        /// Delegates a bounded geospatial query through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot context with caller grants.
        /// - `request`: Typed query criteria and exact source identities.
        ///
        /// # Returns
        /// Matching feature summaries and source provenance.
        ///
        /// # Errors
        /// Returns the delegated query error.
        fn query_geospatial_features(
            &self,
            context: &scriberig_app::ApplicationContext,
            request: &scriberig_app::GeoFeatureQuery,
        ) -> Result<scriberig_app::GeoFeatureQueryResult, scriberig_app::ApplicationError> {
            (**self).query_geospatial_features(context, request)
        }

        /// Delegates a snapshot-bound `GeoPackage` layer catalog through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `path`: Project-relative `GeoPackage` source path.
        /// - `offset`: Zero-based layer offset.
        /// - `limit`: Maximum layer page size.
        ///
        /// # Returns
        /// Bounded provider-neutral dataset and layer metadata.
        ///
        /// # Errors
        /// Returns the delegated service error.
        fn inspect_geopackage(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            path: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::GeoPackageCatalogPage, scriberig_app::ApplicationError> {
            (**self).inspect_geopackage(context, project, path, offset, limit)
        }

        /// Delegates exact-snapshot QGIS project reads through a borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context.
        /// - `project`: Serialized project identity.
        /// - `path`: Exact project-relative QGS/QGZ path.
        ///
        /// # Returns
        /// QGIS layer-reference semantics with exact linked layer IDs.
        ///
        /// # Errors
        /// Returns the delegated application failure.
        fn inspect_qgis_project(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            path: &str,
        ) -> Result<scriberig_app::QgisProjectResult, scriberig_app::ApplicationError> {
            (**self).inspect_qgis_project(context, project, path)
        }

        /// Delegates a snapshot-bound document tree read through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `document`: Serialized document identity.
        ///
        /// # Returns
        /// Typed semantic document tree.
        ///
        /// # Errors
        /// Returns the delegated authorization or project read error.
        fn inspect_document(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            snapshot: &str,
            document: &str,
        ) -> Result<scriberig_app::DocumentTreeView, scriberig_app::ApplicationError> {
            (**self).inspect_document(context, project, snapshot, document)
        }

        /// Delegates a CAD drawing read through the borrowed application service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `drawing`: Serialized semantic drawing identity.
        ///
        /// # Returns
        /// Typed CAD drawing summary.
        ///
        /// # Errors
        /// Returns the delegated authorization or project read error.
        fn inspect_cad_drawing(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            snapshot: &str,
            drawing: &str,
        ) -> Result<scriberig_project::CadDrawingView, scriberig_app::ApplicationError> {
            (**self).inspect_cad_drawing(context, project, snapshot, drawing)
        }

        /// Delegates a bounded CAD entity page through the borrowed application service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `drawing`: Serialized semantic drawing identity.
        /// - `offset`: Zero-based entity offset.
        /// - `limit`: Maximum requested entity count.
        ///
        /// # Returns
        /// Typed CAD entity page.
        ///
        /// # Errors
        /// Returns the delegated authorization or project read error.
        fn read_cad_entities(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            snapshot: &str,
            drawing: &str,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_project::CadEntityPage, scriberig_app::ApplicationError> {
            (**self).read_cad_entities(context, project, snapshot, drawing, offset, limit)
        }

        /// Delegates bounded authored CAD text search through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `request`: Exact-snapshot drawing, query, and bounded page parameters.
        ///
        /// # Returns
        /// Matching semantic entities and authored text.
        ///
        /// # Errors
        /// Returns the delegated authorization or project search error.
        fn find_cad_text(
            &self,
            context: &scriberig_app::ApplicationContext,
            request: &scriberig_app::CadTextSearchRequest,
        ) -> Result<scriberig_project::CadTextSearch, scriberig_app::ApplicationError> {
            (**self).find_cad_text(context, request)
        }

        /// Delegates a CAD bounds query through the borrowed application service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `request`: Exact-snapshot semantic spatial query.
        ///
        /// # Returns
        /// Bounded semantic entities and extent completeness metadata.
        ///
        /// # Errors
        /// Returns the delegated authorization or project query error.
        fn read_cad_bounds_query(
            &self,
            context: &scriberig_app::ApplicationContext,
            request: scriberig_project::CadBoundsQueryRequest,
        ) -> Result<scriberig_project::CadBoundsEntityPage, scriberig_app::ApplicationError> {
            (**self).read_cad_bounds_query(context, request)
        }
    };
}

//! Provides bounded geospatial feature reads over the protocol client.

use scriberig_protocol::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoCatalogRequest, GeoCatalogResponse,
    GeoFeatureQueryRequest, GeoFeatureQueryResponse, GeoFeaturesRequest, GeoFeaturesResponse,
    GeoGeometry, GeoGeometryEditRequest, GeoGeometryEditResponse, QgisLayerOrderRequest,
    QgisLayerOrderResponse, QgisLayerRenameRequest, QgisLayerRenameResponse, QgisProjectRequest,
    QgisProjectResponse, Request, Response,
};

use super::Client;
use crate::error::ClientError;

impl<T: scriberig_protocol::ProtocolTransport> Client<T> {
    /// Proposes an exact-snapshot QGIS project layer-order candidate.
    ///
    /// # Parameters
    /// - `request`: Exact project, snapshot, path, and complete layer order.
    ///
    /// # Returns
    /// Candidate handle for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn propose_qgis_layer_order(
        &self,
        request: QgisLayerOrderRequest,
    ) -> Result<QgisLayerOrderResponse, ClientError> {
        match self.send(Request::QgisLayerOrder(request))?.response {
            Response::QgisLayerOrder(candidate) => Ok(candidate),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Inspects a QGIS project and its resolved layer references at an exact snapshot.
    ///
    /// # Parameters
    /// - `request`: Exact project, snapshot, and QGIS source path.
    ///
    /// # Returns
    /// QGIS structure with provider-neutral shared `GeoLayer` identities.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn inspect_qgis_project(
        &self,
        request: QgisProjectRequest,
    ) -> Result<QgisProjectResponse, ClientError> {
        match self.send(Request::QgisProject(request))?.response {
            Response::QgisProject(project) => Ok(project),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes an exact-snapshot QGIS layer-reference rename candidate.
    ///
    /// # Parameters
    /// - `request`: Exact project, snapshot, semantic reference, and new display name.
    ///
    /// # Returns
    /// Candidate handle for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn propose_qgis_layer_rename(
        &self,
        request: QgisLayerRenameRequest,
    ) -> Result<QgisLayerRenameResponse, ClientError> {
        match self.send(Request::QgisLayerRename(request))?.response {
            Response::QgisLayerRename(candidate) => Ok(candidate),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Executes a bounded typed geospatial query through the remote application API.
    ///
    /// # Parameters
    /// - `request`: Exact project and snapshot query contract.
    ///
    /// # Returns
    /// Matching features and exact source provenance.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn query_geospatial_features(
        &self,
        request: GeoFeatureQueryRequest,
    ) -> Result<GeoFeatureQueryResponse, ClientError> {
        match self.send(Request::GeoFeatureQuery(request))?.response {
            Response::GeoFeatureQuery(result) => Ok(result),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes one typed feature attribute edit as an exact-snapshot candidate.
    ///
    /// # Parameters
    /// - `request`: Exact-snapshot source, feature, field, and replacement value.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn propose_geo_attribute_edit(
        &self,
        request: GeoAttributeEditRequest,
    ) -> Result<GeoAttributeEditResponse, ClientError> {
        match self.send(Request::GeoAttributeEdit(request))?.response {
            Response::GeoAttributeEdit(candidate) => Ok(candidate),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes a typed geometry edit as an exact-snapshot candidate.
    ///
    /// # Parameters
    /// - `project`: Stable owning project identity.
    /// - `path`: Project-relative `GeoPackage` path from the artifact manifest.
    /// - `snapshot`: Exact immutable base snapshot identity.
    /// - `layer`: Stable semantic layer identity.
    /// - `feature`: Stable semantic feature identity.
    /// - `geometry`: Replacement geometry in the layer CRS.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn propose_geo_geometry_edit(
        &self,
        project: impl Into<String>,
        path: impl Into<String>,
        snapshot: impl Into<String>,
        layer: impl Into<String>,
        feature: impl Into<String>,
        geometry: GeoGeometry,
    ) -> Result<GeoGeometryEditResponse, ClientError> {
        match self
            .send(Request::GeoGeometryEdit(GeoGeometryEditRequest {
                project: project.into(),
                path: path.into(),
                snapshot: snapshot.into(),
                layer: layer.into(),
                feature: feature.into(),
                geometry,
            }))?
            .response
        {
            Response::GeoGeometryEdit(candidate) => Ok(candidate),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads bounded `GeoPackage` dataset and layer metadata at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable owning project identity.
    /// - `path`: Project-relative `GeoPackage` path from the artifact manifest.
    /// - `snapshot`: Exact immutable semantic snapshot identity.
    /// - `offset`: Zero-based layer offset.
    /// - `limit`: Requested page size from one through two hundred.
    ///
    /// # Returns
    /// Typed dataset and layer metadata with paging and source-digest evidence.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn inspect_geopackage(
        &self,
        project: impl Into<String>,
        path: impl Into<String>,
        snapshot: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<GeoCatalogResponse, ClientError> {
        match self
            .send(Request::GeoCatalog(GeoCatalogRequest {
                project: project.into(),
                path: path.into(),
                snapshot: snapshot.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::GeoCatalog(catalog) => Ok(catalog),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads a bounded `GeoPackage` feature page at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable owning project identity.
    /// - `path`: Project-relative `GeoPackage` path from the persisted artifact manifest.
    /// - `snapshot`: Exact immutable semantic snapshot identity.
    /// - `layer`: Stable semantic layer identity.
    /// - `offset`: Zero-based feature offset.
    /// - `limit`: Requested page size from one through one thousand.
    ///
    /// Provider-neutral typed feature page.
    ///
    /// # Returns
    /// The bounded feature page with exact project, snapshot, and layer context.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response error.
    pub fn geo_features(
        &self,
        project: impl Into<String>,
        path: impl Into<String>,
        snapshot: impl Into<String>,
        layer: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<GeoFeaturesResponse, ClientError> {
        match self
            .send(Request::GeoFeatures(GeoFeaturesRequest {
                project: project.into(),
                path: path.into(),
                snapshot: snapshot.into(),
                layer: layer.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::GeoFeatures(page) => Ok(page),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}

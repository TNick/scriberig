//! Verifies shared projection style requests across the reusable client boundary.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, InMemoryProjectionStyleStore, InspectRequest, InspectResult,
    KindDispatchStyleAdapter, ProjectionStyleBackend, ProjectionStyleConfiguration,
    RuntimeCapability, StyleResolutionRequest,
};
use scriberig_client::Client;
use scriberig_protocol::{
    ProjectionStyleApplicability, ProjectionStyleFidelity, ProjectionStyleFidelityReport,
    ProjectionStyleProperty, ProjectionStylePurpose, ProjectionStyleReference,
    ProjectionStyleResolveRequest, ProjectionStyleResolveResponse, ProjectionStyleSchemaReference,
    ProjectionStyleSchemaResponse, ProjectionStyleScope, ProjectionStyleSettingRequest,
    ProjectionStyleSettingResponse, ProjectionStyleSource, ProjectionStyleValueKind,
};
use scriberig_server::{ApplicationService, InProcessTransport};
use std::sync::Arc;

/// Provides stable style responses for protocol/client round-trip coverage.
struct StyleFixture;

impl ApplicationService for StyleFixture {
    /// Rejects semantic inspection, which is outside these style cases.
    ///
    /// - `_context`: Trusted application request context.
    /// - `_request`: Semantic inspection request.
    ///
    /// This fixture never returns an inspection result.
    ///
    /// # Returns
    /// An unsupported-operation error.
    ///
    /// # Errors
    /// Always returns `UnsupportedOperation`.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture inspection".to_owned(),
        })
    }

    /// Reports no capabilities for this style-only fixture.
    ///
    /// # Returns
    /// An empty capability list.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Rejects operation lookup because the fixture creates no operations.
    ///
    /// - `_operation_id`: Operation identity.
    ///
    /// This fixture never returns an operation.
    ///
    /// # Returns
    /// An unsupported-operation error.
    ///
    /// # Errors
    /// Always returns `UnsupportedOperation`.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture operation status".to_owned(),
        })
    }

    /// Returns a style schema for the requested projection kind.
    ///
    /// - `_context`: Trusted application request context.
    /// - `projection_kind`: Projection kind requested by the client.
    ///
    /// The schema describing one fixture style property.
    ///
    /// # Returns
    /// A typed style schema response.
    ///
    /// # Errors
    /// This fixture does not fail schema lookup.
    fn projection_style_schema(
        &self,
        _context: &ApplicationContext,
        projection_kind: &str,
    ) -> Result<Option<ProjectionStyleSchemaResponse>, ApplicationError> {
        Ok(Some(ProjectionStyleSchemaResponse {
            reference: ProjectionStyleSchemaReference {
                adapter_id: "fixture".to_owned(),
                version: "1".to_owned(),
            },
            projection_kinds: vec![projection_kind.to_owned()],
            properties: vec![ProjectionStyleProperty {
                key: "indent-width".to_owned(),
                label: "Indent width".to_owned(),
                description: None,
                kind: ProjectionStyleValueKind::Integer,
                allowed_values: Vec::new(),
                minimum: Some(2),
                maximum: Some(8),
                default: b"2".to_vec(),
                applicability: ProjectionStyleApplicability::Both,
                affects_fidelity: false,
                required_for_materialization: false,
            }],
        }))
    }

    /// Returns exact resolved bytes and a lossless fidelity classification.
    ///
    /// - `_context`: Trusted application request context.
    /// - `request`: Purpose and scope selected by the client.
    ///
    /// A stable style response echoing the requested purpose.
    ///
    /// # Returns
    /// The effective style with source and fidelity metadata.
    ///
    /// # Errors
    /// This fixture does not fail style resolution.
    fn resolve_projection_style(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleResolveRequest,
    ) -> Result<ProjectionStyleResolveResponse, ApplicationError> {
        Ok(ProjectionStyleResolveResponse {
            purpose: request.purpose,
            reference: ProjectionStyleReference::Inline,
            value: b"indent-width=2".to_vec(),
            schema: Some(ProjectionStyleSchemaReference {
                adapter_id: "fixture".to_owned(),
                version: "1".to_owned(),
            }),
            applied: vec![ProjectionStyleSource {
                kind: "domain-default".to_owned(),
                identity: "fixture".to_owned(),
                revision: None,
                generation: None,
                value: b"indent-width=2".to_vec(),
            }],
            fidelity: ProjectionStyleFidelityReport {
                fidelity: ProjectionStyleFidelity::LosslessEditable,
                omitted_categories: Vec::new(),
                explanation: None,
            },
        })
    }

    /// Returns the requested style setting as the authoritative stored value.
    ///
    /// - `_context`: Trusted application request context.
    /// - `request`: Scope, kind, style bytes, and expected revision.
    ///
    /// The requested value with its next revision.
    ///
    /// # Returns
    /// The setting result or empty state when cleared.
    ///
    /// # Errors
    /// This fixture does not fail style setting writes.
    fn set_projection_style(
        &self,
        _context: &ApplicationContext,
        request: &ProjectionStyleSettingRequest,
    ) -> Result<ProjectionStyleSettingResponse, ApplicationError> {
        Ok(ProjectionStyleSettingResponse {
            revision: request
                .style
                .as_ref()
                .map(|_| request.expected_revision.unwrap_or(0) + 1),
            style: request.style.clone(),
        })
    }
}

/// Verifies schema discovery, effective style, and setting writes use typed client calls.
#[test]
fn projection_style_operations_round_trip_through_client_and_dispatcher() {
    let client = Client::new(InProcessTransport::new(StyleFixture));
    let schema = client
        .projection_style_schema("javascript")
        .expect("schema response")
        .expect("fixture schema");
    assert_eq!(schema.reference.adapter_id, "fixture");
    assert_eq!(schema.properties[0].key, "indent-width");

    let effective = client
        .resolve_projection_style(ProjectionStyleResolveRequest {
            projection_kind: "javascript".to_owned(),
            project_id: "project-1".to_owned(),
            workspace_id: "workspace-1".to_owned(),
            purpose: ProjectionStylePurpose::InteractiveView,
            view_id: Some("view-1".to_owned()),
            explicit_personal: None,
        })
        .expect("effective style response");
    assert_eq!(effective.value, b"indent-width=2");
    assert_eq!(effective.applied[0].kind, "domain-default");
    assert_eq!(
        effective.fidelity.fidelity,
        ProjectionStyleFidelity::LosslessEditable
    );

    let setting = client
        .set_projection_style(ProjectionStyleSettingRequest {
            scope: ProjectionStyleScope::User,
            projection_kind: "javascript".to_owned(),
            expected_revision: Some(4),
            style: Some(b"quote=single".to_vec()),
        })
        .expect("style setting response");
    assert_eq!(setting.style, Some(b"quote=single".to_vec()));
    assert_eq!(setting.revision, Some(5));
}

/// Supplies only the universal backend call required by an application facade.
struct EmptyBackend;

impl ApplicationBackend for EmptyBackend {
    /// Rejects universal semantic requests outside style resolution coverage.
    ///
    /// - `_context`: Trusted application request context.
    /// - `_request`: Universal semantic request.
    ///
    /// This fixture does not provide canonical semantic execution.
    ///
    /// # Returns
    /// An unsupported-operation error.
    ///
    /// # Errors
    /// Always returns `UnsupportedOperation`.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "empty style fixture".to_owned(),
        })
    }
}

/// Verifies remote style resolution and mutation match the embedded shared service.
#[test]
fn local_and_remote_ecmascript_style_resolution_are_equivalent() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let adapter: Arc<dyn sr_projection::ProjectionStyleAdapter> =
        Arc::new(scriberig_domain_ecma::EcmaStyleAdapter);
    let adapters = KindDispatchStyleAdapter::new()
        .with_adapter("javascript", Arc::clone(&adapter))
        .with_adapter("typescript", adapter);
    let service =
        scriberig_app::ApplicationService::new(EmptyBackend).with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(adapters), store),
        );
    let local_service = service.clone();
    let context = ApplicationContext {
        principal: Some(scriberig_app::Principal::new("alice")),
        user_preferences_principal: Some("alice".to_owned()),
        ..ApplicationContext::empty()
    };
    let client = Client::new(InProcessTransport::for_principal(
        service,
        scriberig_app::Principal::new("alice"),
    ));

    let schema = client
        .projection_style_schema("javascript")
        .expect("remote schema query")
        .expect("ECMAScript domain schema");
    assert_eq!(schema.reference.adapter_id, "ecma-source-style");
    assert_eq!(schema.properties.len(), 3);

    let setting = client
        .set_projection_style(ProjectionStyleSettingRequest {
            scope: ProjectionStyleScope::User,
            projection_kind: "javascript".to_owned(),
            expected_revision: None,
            style: Some(b"quote=single".to_vec()),
        })
        .expect("remote user preference write");
    assert_eq!(setting.style, Some(b"quote=single".to_vec()));
    assert_eq!(setting.revision, Some(1));

    let mut request = StyleResolutionRequest::new(
        sr_projection::ProjectionKind::new("javascript").expect("projection kind"),
        "project-1",
        "workspace-1",
        sr_projection::ProjectionPurpose::InteractiveView,
    );
    request.principal = Some("alice".to_owned());
    let local =
        ProjectionStyleBackend::projection_style_effective(&local_service, &context, &request)
            .expect("local shared style resolution");
    let remote = client
        .resolve_projection_style(ProjectionStyleResolveRequest {
            projection_kind: "javascript".to_owned(),
            project_id: "project-1".to_owned(),
            workspace_id: "workspace-1".to_owned(),
            purpose: ProjectionStylePurpose::InteractiveView,
            view_id: None,
            explicit_personal: None,
        })
        .expect("remote shared style resolution");

    assert_eq!(remote.value, local.resolved.value().as_bytes());
    assert_eq!(remote.applied.len(), local.applied.len());
    assert_eq!(remote.applied[0].kind, local.applied[0].kind.to_string());
    assert_eq!(remote.applied[0].generation, local.applied[0].generation);
    assert_eq!(
        remote.fidelity.fidelity,
        ProjectionStyleFidelity::LosslessEditable
    );
    assert!(local.fidelity.fidelity.is_editable());
}

//! Provides the application fixture used by projection style integration tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InMemoryProjectionStyleStore, Principal,
    ProjectionStyleBackend, ProjectionStyleConfiguration, ProjectionStyleSettingRequest,
    ProjectionStyleStore, StyleEntryKey, StyleResolutionRequest, StyleScopeKind,
};
use sr_projection::{ProjectionPurpose, ProjectionStyleAdapter};

#[path = "projection_style_support.rs"]
pub mod style_support;

pub use style_support::{DocumentStyleAdapter, field, fields, kind, style};

/// Backend spy that records any semantic application request it receives.
#[derive(Clone, Default)]
pub struct RecordingBackend {
    /// Count of semantic requests dispatched through the backend.
    requests: Arc<AtomicUsize>,
}

impl RecordingBackend {
    /// Returns how many semantic requests the spy has seen.
    ///
    /// # Returns
    /// The recorded semantic request count.
    #[must_use]
    pub fn seen(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

impl ApplicationBackend for RecordingBackend {
    /// Records and rejects one universal request.
    ///
    /// - `self`: Backend spy counting requests.
    /// - `_context`: Caller context, unused by the spy.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// # Returns
    /// Never returns a universal result.
    ///
    /// # Errors
    /// Always returns unsupported-operation after counting the request.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        Err(ApplicationError::UnsupportedOperation {
            operation: "test backend".to_owned(),
        })
    }
}

/// Builds one service with the document style adapter installed.
///
/// # Returns
/// The service, its style store, and a backend probe counting semantic requests.
#[must_use]
pub fn service() -> (
    ApplicationService<RecordingBackend>,
    Arc<InMemoryProjectionStyleStore>,
    RecordingBackend,
) {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let backend = RecordingBackend::default();
    let probe = backend.clone();
    let service = ApplicationService::new(backend).with_projection_style_configuration(
        ProjectionStyleConfiguration::new(Arc::new(DocumentStyleAdapter), store.clone()),
    );
    (service, store, probe)
}

/// Builds one caller context for one trusted personal principal.
///
/// - `principal`: Trusted principal identity to attach.
///
/// # Returns
/// The application context for that principal.
#[must_use]
pub fn context_for(principal: &str) -> ApplicationContext {
    ApplicationContext {
        user_preferences_principal: Some(principal.to_owned()),
        principal: Some(Principal::new(principal)),
        ..ApplicationContext::empty()
    }
}

/// Stores one style setting directly in the configuration store.
///
/// - `store`: Configuration store to write.
/// - `scope`: Configuration scope to write.
/// - `identity`: Scope identity whose setting is written.
/// - `pairs`: Style fields to store.
///
/// # Panics
/// Panics when the store rejects the unconditional write.
pub fn store_style(
    store: &InMemoryProjectionStyleStore,
    scope: StyleScopeKind,
    identity: &str,
    pairs: &[(&str, &str)],
) {
    store
        .compare_and_swap(
            &StyleEntryKey::new(scope, identity, kind()),
            None,
            style(pairs),
        )
        .expect("style setting stored");
}

/// Builds one style-set request for the calling principal.
///
/// - `pairs`: Style fields to write, or empty for a clear.
///
/// # Returns
/// The setting request.
#[must_use]
pub fn set_style(pairs: &[(&str, &str)]) -> ProjectionStyleSettingRequest {
    ProjectionStyleSettingRequest::new(kind(), "caller-ignored", Some(style(pairs)))
}

/// Resolves the effective style fields for one purpose, principal, and view.
///
/// - `service`: Application service resolving the style.
/// - `context`: Caller context supplying the trusted principal.
/// - `purpose`: Why the style is being resolved.
/// - `view_id`: View session identity, when the resolution targets one view.
///
/// # Returns
/// The effective style fields for that purpose.
///
/// # Panics
/// Panics when the fixture resolution fails.
#[must_use]
pub fn effective_fields(
    service: &ApplicationService<RecordingBackend>,
    context: &ApplicationContext,
    purpose: ProjectionPurpose,
    view_id: Option<&str>,
) -> BTreeMap<String, String> {
    let mut request = StyleResolutionRequest::new(kind(), "proj-1", "ws-1", purpose);
    request.view_id = view_id.map(ToOwned::to_owned);
    let effective = service
        .projection_style_effective(context, &request)
        .expect("effective style resolves");
    fields(effective.resolved.value())
}

#[cfg(test)]
mod fixture_contracts {
    use super::*;
    use scriberig_app::{
        ApplicationBackend, ApplicationError, ApplicationRequest, CommitRequest,
        ProjectionStyleBackend,
    };

    /// Verifies the fixture adapter validates and merges its own style fields.
    #[test]
    fn the_fixture_adapter_validates_and_merges_domain_fields() {
        let adapter = DocumentStyleAdapter;
        let base = style(&[("indent", "spaces"), ("width", "80")]);
        let overlay = style(&[("width", "100")]);

        let merged = adapter
            .merge(&kind(), &base, &overlay)
            .expect("the domain merges a defined field");

        assert_eq!(field(&fields(&merged), "width"), Some("100"));
        assert_eq!(field(&fields(&merged), "indent"), Some("spaces"));
        assert!(
            adapter
                .normalize(&kind(), &style(&[("indentation", "spaces")]))
                .is_err(),
            "the fixture adapter rejects undefined fields"
        );
    }

    /// Verifies the backend probe stays at zero through style reads and writes.
    #[test]
    fn the_backend_probe_stays_idle_for_style_operations() {
        let (service, _store, probe) = service();
        let context = context_for("alice");

        assert_eq!(
            set_style(&[("width", "100")]).style.as_ref().map(fields),
            Some(fields(&style(&[("width", "100")])))
        );
        service
            .set_projection_style_view_override(
                &context,
                &ProjectionStyleSettingRequest::new(
                    kind(),
                    "view-a",
                    Some(style(&[("emphasis", "italic")])),
                ),
            )
            .expect("view override written");
        assert_eq!(
            field(
                &effective_fields(
                    &service,
                    &context,
                    ProjectionPurpose::InteractiveView,
                    Some("view-a")
                ),
                "emphasis"
            ),
            Some("italic"),
            "the fixture override applies to its own view"
        );

        assert_eq!(
            probe.seen(),
            0,
            "style operations never reach the semantic backend"
        );
    }

    /// Verifies the spy backend counts a semantic request when one is issued.
    #[test]
    fn the_backend_probe_counts_a_semantic_request() {
        let backend = RecordingBackend::default();
        let request = ApplicationRequest::Commit(CommitRequest {
            base_snapshot: scriberig_core::SnapshotId::from_bytes([0; 32]),
            branch: None,
            change: "change-1".to_owned(),
        });

        let error = ApplicationBackend::execute(&backend, &ApplicationContext::empty(), &request)
            .expect_err("the fixture backend supports no universal request");

        assert!(matches!(
            error,
            ApplicationError::UnsupportedOperation { .. }
        ));
        assert_eq!(backend.seen(), 1);
    }
}

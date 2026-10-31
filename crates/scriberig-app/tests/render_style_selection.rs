//! Proves a render carries an explicit style the application resolves.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;
#[path = "universal_operation_support.rs"]
mod support;

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, CompositeBackend, InMemoryProjectionStyleStore, KindDispatchStyleAdapter,
    OperationSummary, ProjectionStyleConfiguration, ProjectionStyleStore, RenderRequest,
    RenderStylePurpose, RenderStyleSelection, SnapshotSelector, StyleEntryKey, StyleScopeKind,
};
use scriberig_core::SnapshotId;
use sr_projection::StyleValue;

use fixture::{DocumentStyleAdapter, context_for, kind, style};
use support::{OperationFixture, ReadFixture};

/// Backend that reports the exact style bytes and provenance it was asked to
/// render with, so a test can see what actually reached the provider.
///
/// # Public members
/// - `dispatches`: Number of universal requests received.
///
/// # Private members
/// - None.
#[derive(Clone, Default)]
struct EchoStyleBackend {
    /// Number of universal requests received.
    dispatches: Arc<std::sync::atomic::AtomicUsize>,
}

impl EchoStyleBackend {
    /// Reports how many requests this backend received.
    ///
    /// # Returns
    /// The recorded request count.
    #[must_use]
    fn seen(&self) -> usize {
        self.dispatches.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ApplicationBackend for EchoStyleBackend {
    /// Echoes one render's applied style back through the operation summary.
    ///
    /// - `self`: Backend echoing the render style.
    /// - `_context`: Caller context, unused by the echo.
    /// - `request`: Universal request to echo or reject.
    ///
    /// # Returns
    /// A render summary carrying the applied style bytes and provenance labels.
    ///
    /// # Errors
    /// Returns unsupported-operation for any non-render request.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.dispatches
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match request {
            ApplicationRequest::Render(render) => {
                let value = String::from_utf8_lossy(render.style.applied_value().unwrap_or(&[]))
                    .to_string();
                let provenance = match &render.style {
                    RenderStyleSelection::Applied { provenance, .. } => provenance.join("+"),
                    _ => "unresolved".to_owned(),
                };
                Ok(ApplicationResult::Render(OperationSummary {
                    handle: format!("{value}|{provenance}"),
                    snapshot: Some(SnapshotId::new([1; 32])),
                    revision: None,
                    branch: None,
                    diff: None,
                    history: None,
                    rendered: None,
                    validation: None,
                    validation_profile: None,
                    validation_diagnostics: Vec::new(),
                    review_details: Vec::new(),
                    conflicts: Vec::new(),
                }))
            }
            _ => Err(ApplicationError::UnsupportedOperation {
                operation: "echo backend".to_owned(),
            }),
        }
    }
}

/// Builds one render request for one style selection.
///
/// - `selection`: Style selection the caller carries.
///
/// # Returns
/// The render request.
fn render_request(selection: RenderStyleSelection) -> ApplicationRequest {
    ApplicationRequest::Render(RenderRequest {
        snapshot: SnapshotSelector::Exact(SnapshotId::new([1; 32])),
        view: "document".to_owned(),
        style: selection,
    })
}

/// Builds one service with the document style adapter over `store`.
///
/// - `store`: Configuration store the service reads.
///
/// # Returns
/// The service and its echo backend.
fn service_with(
    store: Arc<InMemoryProjectionStyleStore>,
) -> (
    scriberig_app::ApplicationService<EchoStyleBackend>,
    EchoStyleBackend,
) {
    let backend = EchoStyleBackend::default();
    let probe = backend.clone();
    let service =
        scriberig_app::ApplicationService::new(backend).with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(DocumentStyleAdapter), store),
        );
    (service, probe)
}

/// Reads the echoed handle out of one application result.
///
/// - `result`: Result to read.
///
/// # Returns
/// The summary handle carrying the applied style and provenance.
fn handle(result: ApplicationResult) -> String {
    match result {
        ApplicationResult::Render(summary) => summary.handle,
        other => panic!("expected a render result, got {other:?}"),
    }
}

/// Stores one style setting directly in the configuration store.
///
/// - `store`: Store to write.
/// - `scope`: Configuration scope to write.
/// - `identity`: Scope identity whose setting is written.
/// - `pairs`: Style fields to store.
///
/// # Panics
/// Panics when the store rejects the unconditional write.
fn store_setting(
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

/// Verifies two explicit styles render one semantic target differently, and that
/// both reach the provider already resolved.
#[test]
fn two_explicit_styles_render_the_same_target_differently() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let (service, backend) = service_with(store);
    let context = context_for("alice");

    let narrow = service
        .execute(
            &context,
            &render_request(RenderStyleSelection::exact(
                style(&[("width", "80")]).as_bytes().to_vec(),
            )),
        )
        .expect("narrow style renders");
    let wide = service
        .execute(
            &context,
            &render_request(RenderStyleSelection::exact(
                style(&[("width", "100")]).as_bytes().to_vec(),
            )),
        )
        .expect("wide style renders");

    assert_ne!(
        handle(narrow.clone()),
        handle(wide.clone()),
        "{narrow:?} vs {wide:?}"
    );
    assert!(handle(narrow).contains("caller-exact-style"));
    assert!(handle(wide).contains("caller-exact-style"));
    assert_eq!(backend.seen(), 2, "both renders reached the provider");
}

/// Verifies a configured selection is resolved by the application into the exact
/// style the provider sees.
#[test]
fn a_configured_selection_is_resolved_before_dispatch() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    store_setting(
        &store,
        StyleScopeKind::Project,
        "proj-1",
        &[("indent", "spaces"), ("width", "100")],
    );
    let (service, _backend) = service_with(store);

    let result = service
        .execute(
            &context_for("alice"),
            &render_request(RenderStyleSelection::configured(
                "document",
                "proj-1",
                "ws-1",
                RenderStylePurpose::InteractiveView,
            )),
        )
        .expect("configured render");
    let echoed = handle(result);

    assert!(
        echoed.contains("\"width\":\"100\""),
        "the provider must see the resolved style, not the selection: {echoed}"
    );
    assert!(
        echoed.contains("ProjectPolicy"),
        "provenance must record which source won: {echoed}"
    );
}

/// Verifies a shared-materialization render never carries a stored personal
/// preference, while an interactive render does.
#[test]
fn a_shared_render_never_carries_a_personal_preference() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    store_setting(&store, StyleScopeKind::User, "alice", &[("width", "100")]);
    let (service, _backend) = service_with(store);

    let shared = handle(
        service
            .execute(
                &context_for("alice"),
                &render_request(RenderStyleSelection::configured(
                    "document",
                    "proj-1",
                    "ws-1",
                    RenderStylePurpose::SharedMaterialization,
                )),
            )
            .expect("shared render"),
    );
    let view = handle(
        service
            .execute(
                &context_for("alice"),
                &render_request(RenderStyleSelection::configured(
                    "document",
                    "proj-1",
                    "ws-1",
                    RenderStylePurpose::InteractiveView,
                )),
            )
            .expect("view render"),
    );

    assert!(
        !shared.contains("UserPreference") && shared.contains("\"width\":\"80\""),
        "a shared render must exclude personal state: {shared}"
    );
    assert!(
        view.contains("UserPreference") && view.contains("\"width\":\"100\""),
        "an interactive view carries the principal's preference: {view}"
    );
}

/// Verifies a projection kind no domain adapter owns is refused before any
/// render is dispatched, instead of receiving a generic style.
#[test]
fn an_unregistered_kind_is_refused_before_dispatch() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let dispatcher = KindDispatchStyleAdapter::new().with_adapter(
        fixture::style_support::KIND,
        Arc::new(DocumentStyleAdapter) as Arc<dyn sr_projection::ProjectionStyleAdapter>,
    );
    let backend = EchoStyleBackend::default();
    let probe = backend.clone();
    let service =
        scriberig_app::ApplicationService::new(backend).with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(dispatcher), store),
        );

    let error = service
        .execute(
            &context_for("alice"),
            &render_request(RenderStyleSelection::configured(
                "unregistered-kind",
                "proj-1",
                "ws-1",
                RenderStylePurpose::InteractiveView,
            )),
        )
        .expect_err("no domain owns this kind");

    match &error {
        ApplicationError::UnsupportedOperation { operation } => {
            assert_eq!(operation, "projection style default");
        }
        other => panic!("expected an unsupported-kind rejection: {other:?}"),
    }
    assert_eq!(
        probe.seen(),
        0,
        "a refused style must not dispatch a render"
    );
}

/// Verifies a provider refuses a render whose style was never resolved by the
/// application, so no provider can style a projection on its own.
#[test]
fn a_provider_refuses_an_unresolved_render_style() {
    let backend = CompositeBackend::new(ReadFixture, OperationFixture::new());
    let context = ApplicationContext::empty();

    let error = scriberig_app::ApplicationBackend::execute(
        &backend,
        &context,
        &render_request(RenderStyleSelection::configured(
            "document",
            "proj-1",
            "ws-1",
            RenderStylePurpose::InteractiveView,
        )),
    )
    .expect_err("a provider must not resolve styles");
    match &error {
        ApplicationError::InvalidRequest { operation, .. } => {
            assert_eq!(operation, "render", "the rejection names the operation");
        }
        other => panic!("expected an invalid-request rejection: {other:?}"),
    }

    let applied = scriberig_app::ApplicationBackend::execute(
        &backend,
        &context,
        &render_request(RenderStyleSelection::Applied {
            value: StyleValue::new(b"{}".to_vec()).as_bytes().to_vec(),
            provenance: vec!["test-fixture".to_owned()],
        }),
    )
    .expect("an applied style renders");
    assert!(matches!(applied, ApplicationResult::Render(_)));
}

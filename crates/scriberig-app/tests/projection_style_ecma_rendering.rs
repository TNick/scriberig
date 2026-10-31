//! Proves application style resolution reaches the ECMAScript source kinds
//! through the real domain adapter, and that one semantic module renders under
//! two exactly resolved styles.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;

use std::path::Path;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationService, EffectiveProjectionStyle, InMemoryProjectionStyleStore,
    KindDispatchStyleAdapter, ProjectionStyleBackend, ProjectionStyleConfiguration,
    ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest, StyleResolutionRequest,
};
use scriberig_domain_ecma::{
    ECMA_SOURCE_KINDS, EcmaStyle, EcmaStyleAdapter, parse_source, render_with_style,
};
use sr_projection::{
    ProjectionFidelity, ProjectionKind, ProjectionPurpose, ProjectionStyleAdapter, StyleSourceKind,
    StyleValue,
};

use fixture::{RecordingBackend, context_for};

/// Module resolved and rendered by every case in this binary.
const MODULE: &str = "\
import ready from \"state\";
export function gather(value, label) {
  const record = label;
  if (value) {
    total = value;
  } else {
    total = \"none\";
  }
  return record;
}
";

/// Builds one projection kind.
///
/// - `label`: Kind label to build.
///
/// # Returns
/// The validated projection kind.
///
/// # Panics
/// Panics when the test kind label is invalid.
fn kind(label: &str) -> ProjectionKind {
    ProjectionKind::new(label).expect("kind label is valid")
}

/// Encodes one raw style field list.
///
/// - `text`: Canonical field text one style source would contribute.
///
/// # Returns
/// The style value carrying those bytes.
fn raw(text: &str) -> StyleValue {
    StyleValue::new(text.as_bytes().to_vec())
}

/// Decodes one resolved style into the renderer's own style type.
///
/// - `effective`: Resolved style the application produced.
///
/// # Returns
/// The style the domain renderer must apply.
///
/// # Panics
/// Panics when a resolved test style is not canonical.
fn style_of(effective: &EffectiveProjectionStyle) -> EcmaStyle {
    EcmaStyle::decode(effective.resolved.value().as_bytes()).expect("resolved style decodes")
}

/// Reads one resolved style back as canonical text.
///
/// - `effective`: Resolved style the application produced.
///
/// # Returns
/// The exact style bytes the renderer would consume.
///
/// # Panics
/// Panics when a resolved style is not utf-8.
fn style_text(effective: &EffectiveProjectionStyle) -> String {
    String::from_utf8(effective.resolved.value().as_bytes().to_vec())
        .expect("resolved style bytes are utf-8")
}

/// Builds one service registering the real ECMAScript domain adapter for both
/// source kinds, exactly as `apps/scriberig/src/projection_style_composition.rs`
/// does in the product composition.
///
/// # Returns
/// The service and its shared style configuration store.
fn ecma_service() -> (
    ApplicationService<RecordingBackend>,
    Arc<InMemoryProjectionStyleStore>,
) {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let adapter: Arc<dyn ProjectionStyleAdapter> = Arc::new(EcmaStyleAdapter);
    let mut dispatcher = KindDispatchStyleAdapter::new();
    for label in ECMA_SOURCE_KINDS {
        dispatcher = dispatcher.with_adapter(*label, Arc::clone(&adapter));
    }
    let service =
        ApplicationService::new(RecordingBackend::default()).with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(dispatcher), store.clone()),
        );
    (service, store)
}

/// Resolves one effective style for one purpose.
///
/// - `service`: Service resolving the style.
/// - `label`: Projection kind being projected.
/// - `purpose`: Why the style is being resolved.
///
/// # Returns
/// The effective style, provenance, and fidelity for that purpose.
///
/// # Panics
/// Panics when resolution fails.
fn effective_for(
    service: &ApplicationService<RecordingBackend>,
    label: &str,
    purpose: ProjectionPurpose,
) -> EffectiveProjectionStyle {
    service
        .projection_style_effective(
            &context_for("alice"),
            &StyleResolutionRequest::new(kind(label), "proj-1", "ws-1", purpose),
        )
        .expect("the registered kind resolves")
}

/// Verifies the application answers schema and default queries for both ECMAScript
/// source kinds and refuses a kind no domain registered.
#[test]
fn the_application_resolves_each_ecma_kind_through_the_domain_adapter() {
    let (service, _store) = ecma_service();

    for label in ECMA_SOURCE_KINDS {
        let schema = service
            .projection_style_schema(
                &context_for("alice"),
                &ProjectionStyleSchemaRequest {
                    projection_kind: kind(label),
                },
            )
            .expect("schema query")
            .expect("the ECMAScript domain declares a schema");
        assert_eq!(
            schema
                .properties
                .iter()
                .map(|property| property.key.as_str())
                .collect::<Vec<_>>(),
            vec!["indent", "indent-width", "quote"],
            "the {label} schema is the domain's own field set"
        );

        let default = effective_for(&service, label, ProjectionPurpose::SharedMaterialization);
        assert_eq!(
            style_text(&default),
            "indent=spaces,indent-width=2,quote=double",
            "the {label} default is the ECMAScript domain default"
        );
        assert_eq!(
            default.fidelity.fidelity,
            ProjectionFidelity::LosslessEditable
        );
    }

    assert!(
        service
            .projection_style_schema(
                &ApplicationContext::empty(),
                &ProjectionStyleSchemaRequest {
                    projection_kind: kind("python"),
                },
            )
            .expect("schema query")
            .is_none(),
        "an unregistered kind receives no generic style"
    );
}

/// Verifies one semantic module renders with different bytes under the two exact
/// styles the application resolves for the same kind and scopes.
#[test]
fn one_ecma_module_renders_under_two_resolved_styles() {
    let (service, _store) = ecma_service();
    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &ProjectionStyleSettingRequest::new(
                kind("javascript"),
                "caller-ignored",
                Some(raw("quote=single")),
            ),
        )
        .expect("personal preference written");
    service
        .set_projection_style_materialization_policy(
            &context_for("alice"),
            &ProjectionStyleSettingRequest::new(
                kind("javascript"),
                "ws-1",
                Some(raw("indent-width=4")),
            ),
        )
        .expect("workspace materialization policy written");

    let view = effective_for(&service, "javascript", ProjectionPurpose::InteractiveView);
    let materialized = effective_for(
        &service,
        "javascript",
        ProjectionPurpose::SharedMaterialization,
    );

    assert_eq!(
        style_text(&view),
        "indent=spaces,indent-width=2,quote=single",
        "the domain merged only the personal field over its default"
    );
    assert_eq!(
        style_text(&materialized),
        "indent=spaces,indent-width=4,quote=double",
        "shared materialization carries the workspace width and never the personal quote"
    );
    assert!(
        view.applied
            .iter()
            .any(|source| source.kind == StyleSourceKind::UserPreference)
    );
    assert!(
        !materialized
            .applied
            .iter()
            .any(|source| source.kind == StyleSourceKind::UserPreference)
    );
    assert_eq!(view.fidelity.fidelity, ProjectionFidelity::LosslessEditable);
    assert_eq!(
        materialized.fidelity.fidelity,
        ProjectionFidelity::LosslessEditable
    );

    let model = parse_source(Path::new("sample.js"), MODULE).expect("module is in the subset");
    let view_text = render_with_style(&model, &style_of(&view)).expect("view style renders");
    let materialized_text =
        render_with_style(&model, &style_of(&materialized)).expect("workspace style renders");

    assert_ne!(view_text, materialized_text);
    assert_ne!(
        view_text, MODULE,
        "the resolved personal style changes presentation bytes"
    );
    assert!(
        view_text.contains("total = 'none'"),
        "personal quoting reached the renderer: {view_text}"
    );
    assert!(
        materialized_text.contains("\n    const record = label;"),
        "workspace width reached the renderer: {materialized_text}"
    );
    assert_eq!(
        parse_source(Path::new("sample.js"), &view_text).expect("view render re-parses"),
        model,
        "the personal style render changed semantic content"
    );
    assert_eq!(
        parse_source(Path::new("sample.js"), &materialized_text)
            .expect("workspace render re-parses"),
        model,
        "the workspace style render changed semantic content"
    );
}

//! Projects shared rendering results into safe read-only terminal content.

use crate::inspector::sanitize;

/// Maximum rendered projection bytes retained by the terminal presentation.
const MAX_PROJECTION_BYTES: usize = 16 * 1024;

/// Visible suffix identifying content clipped at the presentation limit.
const TRUNCATED_SUFFIX: &str = "\n… truncated …";

/// Read-only projection result suitable for terminal rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionModel {
    /// Projection identity, when supplied by the renderer.
    pub projection: Option<String>,
    /// Source semantic entity reference.
    pub source_entity: scriberig_core::EntityId,
    /// Adapter-supplied source path metadata, when applicable.
    pub source_path: Option<String>,
    /// Exact source revision, when supplied.
    pub source_revision: Option<scriberig_core::RevisionId>,
    /// Projection style identity.
    pub style: String,
    /// Renderer identity.
    pub renderer: String,
    /// Bounded rendered text after terminal-control sanitization.
    pub content: String,
    /// Synchronization state known to this read operation.
    pub synchronization: Option<String>,
    /// Last known exact base snapshot.
    pub base_snapshot: scriberig_core::SnapshotId,
}

/// Builds a short safe identity label for one resolved projection style.
///
/// - `style`: Exact resolved style consumed by the renderer.
///
/// A label a terminal may display. It names the adapter schema when the
/// resolution carried one, and otherwise shows a stable digest prefix of the
/// canonical style bytes. Raw style bytes are never shown.
#[must_use]
/// # Returns
/// The presentation label.
fn style_identity(style: &sr_projection::ResolvedStyle) -> String {
    if let Some(schema) = style.schema() {
        return format!("{} v{}", schema.adapter_id(), schema.version());
    }
    let digest = style.value_digest();
    let prefix: String = digest[..6].iter().fold(String::new(), |mut text, byte| {
        use std::fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
        text
    });
    format!("style-{prefix}")
}
impl ProjectionModel {
    /// Builds a safe projection model from a public rendered result.
    #[must_use]
    /// - `rendered`: Argument named `rendered`.
    ///
    /// # Returns
    /// The operation result.
    pub fn from_rendered(rendered: &sr_projection::RenderedProjection) -> Self {
        Self {
            projection: rendered.projection.map(|value| value.0.to_string()),
            source_entity: rendered.cache_key.scope.entity,
            source_path: rendered
                .cache_key
                .context
                .extensions
                .get("scriberig.project.source_path")
                .map(|path| sanitize(&String::from_utf8_lossy(path))),
            source_revision: rendered.cache_key.scope.revision,
            style: style_identity(&rendered.style),
            renderer: rendered.renderer.0.clone(),
            content: bounded_content(&rendered.bytes),
            synchronization: None,
            base_snapshot: rendered.base_state.0,
        }
    }
}

/// Sanitizes only a bounded prefix of rendered projection bytes.
///
/// - `bytes`: Renderer output that may contain untrusted or pathological text.
///
/// Sanitized projection content with an explicit truncation marker when needed.
/// # Returns
/// The bounded safe projection text.
fn bounded_content(bytes: &[u8]) -> String {
    let prefix = &bytes[..bytes.len().min(MAX_PROJECTION_BYTES)];
    let mut content = sanitize(&String::from_utf8_lossy(prefix));
    if bytes.len() > MAX_PROJECTION_BYTES {
        content.push_str(TRUNCATED_SUFFIX);
    }
    content
}

#[cfg(test)]
mod tests {
    use super::{MAX_PROJECTION_BYTES, ProjectionModel, TRUNCATED_SUFFIX};
    use scriberig_core::{EntityId, SnapshotId};
    use sr_projection::{
        ProjectionKind, RenderRequest, RenderedProjection, RendererVersion, RenderingContext,
        ResolvedStyle, SemanticScopeRef, SemanticStateRef, StyleValue,
    };

    /// Verifies rendered terminal control characters are made harmless.
    #[test]
    fn sanitizes_rendered_projection_content() {
        let request = RenderRequest {
            specification: sr_projection::ProjectionSpecRef(EntityId::from_bytes([1; 16])),
            scope: SemanticScopeRef {
                entity: EntityId::from_bytes([2; 16]),
                revision: None,
            },
            base_state: SemanticStateRef(SnapshotId::new([3; 32])),
            kind: ProjectionKind::new("text").expect("kind"),
            style: ResolvedStyle::inline(StyleValue::new(vec![1])),
            renderer: RendererVersion::new("v1"),
            context: RenderingContext {
                extensions: std::collections::BTreeMap::from([(
                    "scriberig.project.source_path".to_owned(),
                    b"src/main.rs".to_vec(),
                )]),
            },
        };
        let model = ProjectionModel::from_rendered(&RenderedProjection::from_bytes(
            b"safe\x1b[31m text".to_vec(),
            &request,
            None,
        ));
        assert!(!model.content.contains('\x1b'));
        assert_eq!(model.source_path.as_deref(), Some("src/main.rs"));
    }

    /// Verifies huge unbroken projection content is clipped before sanitizing.
    #[test]
    fn bounds_large_projection_content_before_terminal_rendering() {
        let request = RenderRequest {
            specification: sr_projection::ProjectionSpecRef(EntityId::from_bytes([1; 16])),
            scope: SemanticScopeRef {
                entity: EntityId::from_bytes([2; 16]),
                revision: None,
            },
            base_state: SemanticStateRef(SnapshotId::new([3; 32])),
            kind: ProjectionKind::new("text").expect("kind"),
            style: ResolvedStyle::inline(StyleValue::new(vec![1])),
            renderer: RendererVersion::new("v1"),
            context: RenderingContext {
                extensions: std::collections::BTreeMap::new(),
            },
        };
        let bytes = vec![b'x'; MAX_PROJECTION_BYTES * 100];
        let rendered = RenderedProjection::from_bytes(bytes, &request, None);

        let model = ProjectionModel::from_rendered(&rendered);

        assert!(model.content.len() <= MAX_PROJECTION_BYTES + TRUNCATED_SUFFIX.len());
        assert!(model.content.ends_with(TRUNCATED_SUFFIX));
    }
}

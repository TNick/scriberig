//! Projects shared rendering results into safe read-only terminal content.

use crate::inspector::sanitize;

/// Read-only projection result suitable for terminal rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionModel {
    /// Projection identity, when supplied by the renderer.
    pub projection: Option<String>,
    /// Source semantic entity reference.
    pub source_entity: scriberig_core::EntityId,
    /// Exact source revision, when supplied.
    pub source_revision: Option<scriberig_core::RevisionId>,
    /// Projection style identity.
    pub style: String,
    /// Renderer identity.
    pub renderer: String,
    /// Rendered text after terminal-control sanitization.
    pub content: String,
    /// Synchronization state known to this read operation.
    pub synchronization: String,
    /// Last known exact base snapshot.
    pub base_snapshot: scriberig_core::SnapshotId,
}

impl ProjectionModel {
    /// Builds a safe projection model from a public rendered result.
    #[must_use]
    pub fn from_rendered(rendered: &sr_projection::RenderedProjection) -> Self {
        Self {
            projection: rendered.projection.map(|value| value.0.to_string()),
            source_entity: rendered.cache_key.scope.entity,
            source_revision: rendered.cache_key.scope.revision,
            style: format!("{:?}", rendered.style),
            renderer: rendered.renderer.0.clone(),
            content: sanitize(&String::from_utf8_lossy(&rendered.bytes)),
            synchronization: "rendered".to_owned(),
            base_snapshot: rendered.base_state.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectionModel;
    use scriberig_core::{EntityId, SnapshotId};
    use sr_projection::{
        ProjectionKind, RenderRequest, RenderedProjection, RendererVersion, RenderingContext,
        SemanticScopeRef, SemanticStateRef, StyleRef, StyleValue,
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
            style: StyleRef::Value(StyleValue::new(vec![1])),
            renderer: RendererVersion::new("v1"),
            context: RenderingContext::default(),
        };
        let model = ProjectionModel::from_rendered(&RenderedProjection::from_bytes(
            b"safe\x1b[31m text".to_vec(),
            &request,
            None,
        ));
        assert!(!model.content.contains('\x1b'));
    }
}

//! Defines deterministic projection render inputs and outputs.

use std::collections::BTreeMap;

use crate::model::{
    ProjectionKind, ProjectionRef, ProjectionSpecRef, RendererVersion, SemanticScopeRef,
};
use crate::{model::SemanticStateRef, style::StyleRef};

/// Supplies explicit immutable and contextual inputs to a renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderRequest {
    /// Persistent projection specification being rendered.
    pub specification: ProjectionSpecRef,
    /// Semantic entity and optional exact revision being rendered.
    pub scope: SemanticScopeRef,
    /// Exact snapshot against which the semantic scope is resolved.
    pub base_state: SemanticStateRef,
    /// Adapter-defined output format.
    pub kind: ProjectionKind,
    /// Exact immutable style input.
    pub style: StyleRef,
    /// Exact renderer implementation version.
    pub renderer: RendererVersion,
    /// Adapter-defined deterministic rendering options.
    pub context: RenderingContext,
}

/// Carries deterministic extension values without universal domain schemas.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderingContext {
    /// Stable-keyed adapter extension values.
    pub extensions: BTreeMap<String, Vec<u8>>,
}

/// Describes exact rendered representation and output hash metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedProjection {
    /// Exact rendered representation bytes.
    pub bytes: Vec<u8>,
    /// Placeholder boundary for a deterministic digest of `bytes`.
    pub output_hash: Option<OutputHash>,
    /// Exact semantic snapshot used for rendering.
    pub base_state: SemanticStateRef,
    /// Exact style input used for rendering.
    pub style: StyleRef,
    /// Exact renderer implementation version used for rendering.
    pub renderer: RendererVersion,
    /// Projection kind used for the output.
    pub kind: ProjectionKind,
    /// Projection instance associated with the output, when known.
    pub projection: Option<ProjectionRef>,
}

/// Represents deterministic output digest metadata without choosing an algorithm.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OutputHash {
    /// Stable algorithm identifier such as an adapter-defined hash name.
    pub algorithm: String,
    /// Digest bytes produced by that algorithm.
    pub digest: Vec<u8>,
}

// Construct the stable digest used by deterministic renderers.
impl RenderedProjection {
    /// Creates a rendered output and computes its BLAKE3 digest.
    ///
    /// # Parameters
    /// - `bytes`: Exact rendered representation.
    /// - `request`: Exact request inputs used to produce the bytes.
    /// - `projection`: Optional projection instance.
    ///
    /// # Returns
    /// A rendered output with its deterministic digest and input metadata.
    #[must_use]
    pub fn from_bytes(
        bytes: Vec<u8>,
        request: &RenderRequest,
        projection: Option<ProjectionRef>,
    ) -> Self {
        let digest = blake3::hash(&bytes).as_bytes().to_vec();
        Self {
            bytes,
            output_hash: Some(OutputHash {
                algorithm: "blake3".to_owned(),
                digest,
            }),
            base_state: request.base_state,
            style: request.style.clone(),
            renderer: request.renderer.clone(),
            kind: request.kind.clone(),
            projection,
        }
    }
}

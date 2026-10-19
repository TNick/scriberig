//! Defines deterministic projection render inputs and outputs.

use std::collections::BTreeMap;

use crate::model::{
    ProjectionKind, ProjectionRef, ProjectionSpecRef, RendererVersion, SemanticScopeRef,
};
use crate::{model::SemanticStateRef, style::StyleRef};

/// Supplies explicit immutable and contextual inputs to a renderer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct RenderingContext {
    /// Stable-keyed adapter extension values.
    pub extensions: BTreeMap<String, Vec<u8>>,
}

/// Exact immutable inputs identifying one render/cache computation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderCacheKey {
    /// Persistent projection specification reference.
    pub specification: ProjectionSpecRef,
    /// Semantic scope being rendered.
    pub scope: SemanticScopeRef,
    /// Exact semantic state used for rendering.
    pub base_state: SemanticStateRef,
    /// Projection kind.
    pub kind: ProjectionKind,
    /// Exact style input.
    pub style: StyleRef,
    /// Exact renderer version.
    pub renderer: RendererVersion,
    /// Deterministic adapter rendering context.
    pub context: RenderingContext,
}

impl RenderCacheKey {
    /// Builds a cache identity from every render request input.
    ///
    /// - `request`: Exact render request.
    ///
    /// A stable cache identity containing all output-affecting inputs.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_request(request: &RenderRequest) -> Self {
        Self {
            specification: request.specification,
            scope: request.scope,
            base_state: request.base_state,
            kind: request.kind.clone(),
            style: request.style.clone(),
            renderer: request.renderer.clone(),
            context: request.context.clone(),
        }
    }
}

/// Carries one adapter-owned reusable intermediate rendering fragment.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderedFragment {
    /// Exact local immutable-input key selected by the adapter.
    pub input_key: Vec<u8>,
    /// Exact fragment bytes.
    pub bytes: Vec<u8>,
    /// Digest of the fragment bytes.
    pub output_hash: OutputHash,
}

impl RenderedFragment {
    /// Creates a reusable fragment from exact local input and output bytes.
    ///
    /// - `input`: Exact local immutable input bytes.
    /// - `bytes`: Exact fragment output bytes.
    ///
    /// A fragment with deterministic input and output metadata.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(input: &[u8], bytes: &[u8]) -> Self {
        Self {
            input_key: blake3::hash(input).as_bytes().to_vec(),
            bytes: bytes.to_vec(),
            output_hash: OutputHash {
                algorithm: "blake3".to_owned(),
                digest: blake3::hash(bytes).as_bytes().to_vec(),
            },
        }
    }
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
    /// Exact global cache identity used for this output.
    pub cache_key: RenderCacheKey,
    /// Optional reusable adapter-owned intermediate fragments.
    pub fragments: Vec<RenderedFragment>,
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
    /// - `bytes`: Exact rendered representation.
    /// - `request`: Exact request inputs used to produce the bytes.
    /// - `projection`: Optional projection instance.
    ///
    /// A rendered output with its deterministic digest and input metadata.
    #[must_use]
    /// # Returns
    /// The operation result.
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
            cache_key: RenderCacheKey::from_request(request),
            fragments: Vec::new(),
        }
    }

    /// Attaches reusable intermediate fragments to this rendered output.
    ///
    /// - `self`: Rendered output receiving the fragments.
    /// - `fragments`: Adapter-owned reusable fragments.
    ///
    /// This output with the supplied fragments attached.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_fragments(mut self, fragments: Vec<RenderedFragment>) -> Self {
        self.fragments = fragments;
        self
    }
}

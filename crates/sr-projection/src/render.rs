//! Defines deterministic projection render inputs and outputs.

use std::collections::BTreeMap;

use crate::{model::ProjectionKind, model::RendererVersion, model::SemanticScopeRef};
use crate::{model::SemanticStateRef, style::StyleRef};

/// Supplies explicit immutable and contextual inputs to a renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderRequest {
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
}

/// Represents deterministic output digest metadata without choosing an algorithm.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OutputHash {
    /// Stable algorithm identifier such as an adapter-defined hash name.
    pub algorithm: String,
    /// Digest bytes produced by that algorithm.
    pub digest: Vec<u8>,
}

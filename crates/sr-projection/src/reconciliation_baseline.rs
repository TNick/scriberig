//! Binds an editable projection to the exact output and semantics it came from.

use crate::{FidelityAssessment, ProjectionRef, RenderCacheKey, RenderRequest, RenderedProjection};
use serde::{Deserialize, Serialize};

/// Carries adapter-owned mapping or loss metadata bound to one exact rendering.
///
/// # Public members
/// - `payload`: Adapter-defined serialized evidence.
///
/// # Private members
/// - `cache_key`: Exact immutable render inputs associated with the evidence.
/// - `output_hash`: Exact output digest associated with the evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectionEvidence {
    /// Adapter-defined serialized mapping, hidden-content, or anchor evidence.
    pub payload: Vec<u8>,
    cache_key: RenderCacheKey,
    output_hash: crate::OutputHash,
}

impl ProjectionEvidence {
    /// Binds adapter evidence to a digest-verified rendered output.
    ///
    /// - `rendered`: Exact rendered bytes and immutable inputs.
    /// - `payload`: Adapter-defined evidence bytes.
    ///
    /// Evidence that can be attached to only this exact render baseline.
    ///
    /// # Returns
    /// The bound evidence when the rendered output hash is valid.
    ///
    /// # Errors
    /// Returns an error when the rendered output digest is missing or invalid.
    pub fn new(rendered: &RenderedProjection, payload: Vec<u8>) -> Result<Self, &'static str> {
        let output_hash = rendered
            .output_hash
            .as_ref()
            .ok_or("rendered evidence has no output digest")?;
        let actual = blake3::hash(&rendered.bytes);
        if output_hash.algorithm != "blake3" || output_hash.digest != actual.as_bytes() {
            return Err("rendered evidence digest does not match exact output bytes");
        }
        Ok(Self {
            payload,
            cache_key: rendered.cache_key.clone(),
            output_hash: output_hash.clone(),
        })
    }

    /// Reports whether evidence is still bound to the supplied exact output.
    ///
    /// - `self`: Evidence to validate.
    /// - `rendered`: Candidate rendered output.
    ///
    /// True only when both render inputs and output identity match.
    ///
    /// # Returns
    /// Whether this evidence matches the supplied render.
    #[must_use]
    pub fn matches(&self, rendered: &RenderedProjection) -> bool {
        self.cache_key == rendered.cache_key
            && rendered.output_hash.as_ref() == Some(&self.output_hash)
    }
}

/// Identifies the exact projection adapter implementation used for a baseline.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ProjectionAdapterIdentity {
    /// Stable adapter identifier.
    pub id: String,
    /// Adapter implementation version.
    pub version: String,
}

impl ProjectionAdapterIdentity {
    /// Creates a validated adapter identity.
    ///
    /// - `id`: Stable adapter identifier.
    /// - `version`: Exact adapter implementation version.
    ///
    /// The identity when both values are nonempty.
    ///
    /// # Returns
    /// The validated adapter identity.
    ///
    /// # Errors
    /// Returns an error when either value is empty or whitespace-only.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Result<Self, &'static str> {
        let id = id.into();
        let version = version.into();
        if id.trim().is_empty() || version.trim().is_empty() {
            return Err("adapter identity and version must not be empty");
        }
        Ok(Self { id, version })
    }
}

/// Carries exact output bytes and metadata required for three-way reconciliation.
///
/// The rendered output already binds bytes to its semantic snapshot, scope,
/// specification, projection kind, resolved style, and renderer cache key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconciliationBaseline {
    /// Exact rendered bytes and every immutable render input.
    pub rendered: RenderedProjection,
    /// Exact adapter implementation that produced the output.
    pub adapter: ProjectionAdapterIdentity,
    /// Editability/fidelity assessment for this exact output.
    pub fidelity: FidelityAssessment,
    /// Optional adapter mapping/loss metadata bound to this exact output.
    evidence: Option<ProjectionEvidence>,
}

impl ReconciliationBaseline {
    /// Returns optional adapter evidence after rechecking its exact binding.
    ///
    /// - `self`: Baseline whose evidence is being accessed.
    ///
    /// Adapter evidence when present and still bound to this render.
    ///
    /// # Returns
    /// The exact-bound evidence or no evidence when none was attached.
    ///
    /// # Errors
    /// Returns an error if the baseline output digest or evidence binding changed.
    pub fn evidence(&self) -> Result<Option<&ProjectionEvidence>, &'static str> {
        let Some(evidence) = &self.evidence else {
            return Ok(None);
        };
        let Some(output_hash) = &self.rendered.output_hash else {
            return Err("rendered baseline has no output digest");
        };
        let actual = blake3::hash(&self.rendered.bytes);
        if output_hash.algorithm != "blake3" || output_hash.digest != actual.as_bytes() {
            return Err("rendered baseline digest does not match exact output bytes");
        }
        if !evidence.matches(&self.rendered) {
            return Err("projection evidence does not match exact render inputs and output");
        }
        Ok(Some(evidence))
    }

    /// Creates a baseline from exact render inputs and output bytes.
    ///
    /// - `request`: Exact semantic and presentation inputs consumed by rendering.
    /// - `projection`: Optional managed projection identity.
    /// - `bytes`: Exact rendered output bytes.
    /// - `adapter`: Adapter identity/version that produced the output.
    /// - `fidelity`: Editability assessment for this exact output.
    ///
    /// The reconciliation baseline for the exact render.
    ///
    /// # Returns
    /// A baseline with no optional adapter evidence.
    #[must_use]
    pub fn from_render(
        request: &RenderRequest,
        projection: Option<ProjectionRef>,
        bytes: Vec<u8>,
        adapter: ProjectionAdapterIdentity,
        fidelity: FidelityAssessment,
    ) -> Self {
        let rendered = RenderedProjection::from_bytes(bytes, request, projection);
        Self {
            rendered,
            adapter,
            fidelity,
            evidence: None,
        }
    }

    /// Creates a baseline only when the rendered output digest matches its bytes.
    ///
    /// - `rendered`: Exact output and render identity.
    /// - `adapter`: Adapter identity/version that produced the output.
    /// - `fidelity`: Exact output's editability assessment.
    ///
    /// The validated reconciliation baseline.
    ///
    /// # Returns
    /// A baseline with verified output identity and optional bound evidence.
    ///
    /// # Errors
    /// Returns an error when the output digest is missing or does not match.
    pub fn new(
        rendered: RenderedProjection,
        adapter: ProjectionAdapterIdentity,
        fidelity: FidelityAssessment,
    ) -> Result<Self, &'static str> {
        Self::new_with_evidence(rendered, adapter, fidelity, None)
    }

    /// Creates a validated baseline with optional exact-render evidence.
    ///
    /// - `rendered`: Exact output and immutable render identity.
    /// - `adapter`: Adapter identity/version that produced the output.
    /// - `fidelity`: Editability assessment for this output.
    /// - `evidence`: Optional adapter-owned evidence tied to this output.
    ///
    /// A reconciliation baseline whose evidence matches its output identity.
    ///
    /// # Returns
    /// The baseline after output and evidence validation.
    ///
    /// # Errors
    /// Returns an error when the output digest is invalid or evidence belongs to
    /// a different render/output.
    pub fn new_with_evidence(
        rendered: RenderedProjection,
        adapter: ProjectionAdapterIdentity,
        fidelity: FidelityAssessment,
        evidence: Option<ProjectionEvidence>,
    ) -> Result<Self, &'static str> {
        if adapter.id.trim().is_empty() || adapter.version.trim().is_empty() {
            return Err("adapter identity and version must not be empty");
        }
        if rendered.cache_key.base_state != rendered.base_state
            || rendered.cache_key.style != rendered.style
            || rendered.cache_key.renderer != rendered.renderer
            || rendered.cache_key.kind != rendered.kind
        {
            return Err("rendered output metadata does not match its cache identity");
        }
        let Some(output_hash) = &rendered.output_hash else {
            return Err("rendered baseline has no output digest");
        };
        let actual = blake3::hash(&rendered.bytes);
        if output_hash.algorithm != "blake3" || output_hash.digest != actual.as_bytes() {
            return Err("rendered baseline digest does not match exact output bytes");
        }
        if evidence
            .as_ref()
            .is_some_and(|value| !value.matches(&rendered))
        {
            return Err("projection evidence does not match exact render inputs and output");
        }
        Ok(Self {
            rendered,
            adapter,
            fidelity,
            evidence,
        })
    }

    /// Replaces exact output bytes after a successful synchronization.
    ///
    /// - `self`: Prior baseline whose immutable style/renderer/scope stay fixed.
    /// - `bytes`: Newly written output bytes.
    /// - `base_state`: Semantic state represented by the new output.
    ///
    /// A new baseline with recomputed output identity.
    ///
    /// # Returns
    /// The updated baseline with adapter evidence cleared until regenerated.
    #[must_use]
    pub fn with_output(&self, bytes: Vec<u8>, base_state: crate::SemanticStateRef) -> Self {
        let mut rendered = self.rendered.clone();
        let digest = blake3::hash(&bytes).as_bytes().to_vec();
        rendered.bytes = bytes;
        rendered.output_hash = Some(crate::OutputHash {
            algorithm: "blake3".to_owned(),
            digest,
        });
        rendered.base_state = base_state;
        rendered.cache_key.base_state = base_state;
        Self {
            rendered,
            adapter: self.adapter.clone(),
            fidelity: self.fidelity.clone(),
            evidence: None,
        }
    }
}

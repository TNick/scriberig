//! Defines the deterministic applied-settings fixture report scenarios.

/// Report shape a fixture provider publishes after preparing a request.
#[derive(Clone, Copy, Debug)]
pub enum ReportMode {
    /// A versioned partial report naming the exact bound model.
    Attested,
    /// A provider-prepared report with a digest of its final JSON body.
    AttestedV2,
    /// A version-2 report with a malformed body digest.
    InvalidV2,
    /// A version-three report with valid body and safe-header digests.
    AttestedV3,
    /// A version-four report with complete safe provider-option evidence.
    AttestedV4,
    /// A version-four report with a transformed provider-option value.
    AttestedV4Transformed,
    /// A version-three report with a malformed safe-header digest.
    InvalidV3,
    /// A version-four report with a provider-option identity from another model.
    InvalidV4,
    /// A complete version-five report with a valid final-request digest.
    AttestedV5,
    /// An incomplete version-five report whose request identity is unsafe.
    PartialV5,
    /// A version-five complete report with a malformed request-effect digest.
    InvalidV5,
    /// A version-five complete report that also declares omitted categories.
    CompleteWithOmissionsV5,
    /// A report naming a model other than the one bound to the branch.
    OtherModel,
    /// A report naming a provider other than the selected provider.
    OtherProvider,
    /// A report naming an API other than the bound model API.
    OtherApi,
    /// A report from an unsupported report contract version.
    UnknownSchema,
    /// An incomplete report that names no omitted category.
    Unattested,
    /// A provider that prepares a body but publishes no report.
    Silent,
}

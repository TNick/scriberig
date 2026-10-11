//! Defines provenance-bearing branch descriptions.
use scriberig_core::Provenance;
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Description {
    pub text: String,
    pub provenance: Provenance,
}

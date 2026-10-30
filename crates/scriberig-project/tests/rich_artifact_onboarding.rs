//! Verifies one mixed project imports office, CAD, QGIS, and geospatial semantics.

#[path = "rich_artifact_onboarding/dxf_fixture.rs"]
mod dxf_fixture;
#[path = "rich_artifact_onboarding/fixture_evidence.rs"]
mod fixture_evidence;
#[path = "rich_artifact_onboarding/mixed_domain_edits.rs"]
mod mixed_domain_edits;
#[path = "rich_artifact_onboarding/mixed_project_import.rs"]
mod mixed_project_import;
#[path = "rich_artifact_onboarding/qgis_layer_order.rs"]
mod qgis_layer_order;
#[path = "rich_artifact_onboarding/qgis_rename.rs"]
mod qgis_rename;
#[path = "rich_artifact_onboarding/semantic_candidate_assertions.rs"]
mod semantic_candidate_assertions;
#[path = "rich_artifact_onboarding/support.rs"]
mod support;

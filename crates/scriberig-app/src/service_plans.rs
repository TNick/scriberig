//! Implements application-owned durable plan operations.

#[path = "service_plans/authoring.rs"]
mod authoring;
#[path = "service_plans/authoring_evolution.rs"]
mod authoring_evolution;
#[path = "service_plans/lifecycle.rs"]
mod lifecycle;
#[path = "service_plans/semantic_mutations.rs"]
mod semantic_mutations;
#[path = "service_plans/validation.rs"]
mod validation;

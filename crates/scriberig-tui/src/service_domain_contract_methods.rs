//! Composes focused rich-domain default methods into the public TUI service trait.

#[macro_use]
#[path = "service_domain_contract_methods_cad.rs"]
mod cad_methods;
#[macro_use]
#[path = "service_domain_contract_methods_document.rs"]
mod document_methods;

macro_rules! tui_domain_methods {
    () => {
        tui_cad_domain_methods!();
        tui_document_artifact_methods!();
    };
}

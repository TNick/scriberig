//! Defines optional shared operations for typed rich-artifact surfaces.

#[macro_use]
#[path = "server_rich_artifact_contract/read.rs"]
mod read;
#[macro_use]
#[path = "server_rich_artifact_contract/write.rs"]
mod write;

macro_rules! shared_rich_artifact_contract {
    () => {
        shared_rich_artifact_read_contract!();
        shared_rich_artifact_edit_contract!();
    };
}

//! Defines the local TUI command descriptors and composer completion lookup.

/// Describes one local command available to the conversation composer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlashCommandDescriptor {
    /// Canonical command spelling, including its leading slash.
    pub name: &'static str,
    /// Short description shared by help and completion surfaces.
    pub summary: &'static str,
    /// Optional argument hint shown by command discovery.
    pub argument_hint: &'static str,
}

/// Commands recognized by the product command parser.
pub const COMMANDS: &[SlashCommandDescriptor] = &[
    SlashCommandDescriptor {
        name: "/edit",
        summary: "Edit the selected source projection semantically",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/model",
        summary: "Choose the session model",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/assign",
        summary: "Assign the selected plan node",
        argument_hint: " [node]",
    },
    SlashCommandDescriptor {
        name: "/thinking",
        summary: "Choose the thinking level",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/theme",
        summary: "Choose the upstream terminal theme",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/view",
        summary: "Change activity presentation",
        argument_hint: " <kind>",
    },
    SlashCommandDescriptor {
        name: "/plan",
        summary: "Create or control a durable plan",
        argument_hint: " <goal|show|next|child|retry|waive|action>",
    },
    SlashCommandDescriptor {
        name: "/session",
        summary: "Open the structural session tree",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/explorer",
        summary: "Open semantic exploration",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/inspect",
        summary: "Inspect the selected entity",
        argument_hint: " [entity] [--snapshot <id>]",
    },
    SlashCommandDescriptor {
        name: "/find",
        summary: "Search semantic content",
        argument_hint: " <query>",
    },
    SlashCommandDescriptor {
        name: "/history",
        summary: "Open snapshot history",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/diff",
        summary: "Compare snapshots",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/projection",
        summary: "Open a semantic projection",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/source-style",
        summary: "Inspect or set your personal Rust source projection style",
        argument_hint: " <show|indent=spaces,indent-width=2,blank-lines=1|clear>",
    },
    SlashCommandDescriptor {
        name: "/changes",
        summary: "Review candidate changes",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/operations",
        summary: "Show application operations",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/project",
        summary: "Show the active project",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/artifacts",
        summary: "Inspect bounded project artifacts and fidelity",
        argument_hint: " [project-id] [offset]",
    },
    SlashCommandDescriptor {
        name: "/artifact-members",
        summary: "Read a bounded page of exact artifact members",
        argument_hint: " <project-id> <artifact-id> [offset]",
    },
    SlashCommandDescriptor {
        name: "/spreadsheet",
        summary: "Read a bounded page from the selected worksheet",
        argument_hint: " [A1:C20] [offset]",
    },
    SlashCommandDescriptor {
        name: "/cad",
        summary: "Inspect a bounded semantic CAD drawing page",
        argument_hint: " [offset]",
    },
    SlashCommandDescriptor {
        name: "/cad-select",
        summary: "Select an entity from the loaded CAD page",
        argument_hint: " <entity-id>",
    },
    SlashCommandDescriptor {
        name: "/cad-layer",
        summary: "Propose moving a CAD entity to another layer",
        argument_hint: " <entity-id> <layer-id>",
    },
    SlashCommandDescriptor {
        name: "/geo",
        summary: "Inspect a GeoPackage catalog at the selected snapshot",
        argument_hint: " <project-relative-path> [layer-offset]",
    },
    SlashCommandDescriptor {
        name: "/qgis",
        summary: "Inspect a QGIS project's layer references at the selected snapshot",
        argument_hint: " <project-relative-path>",
    },
    SlashCommandDescriptor {
        name: "/qgis-layer",
        summary: "Open the GeoPackage catalog linked by a loaded QGIS reference",
        argument_hint: " <reference-id>",
    },
    SlashCommandDescriptor {
        name: "/qgis-rename",
        summary: "Propose renaming a QGIS layer reference for shared review",
        argument_hint: " <reference-id> <new-name>",
    },
    SlashCommandDescriptor {
        name: "/qgis-order",
        summary: "Propose a complete QGIS layer-reference order for shared review",
        argument_hint: " <reference-id> <reference-id> ...",
    },
    SlashCommandDescriptor {
        name: "/geo-features",
        summary: "Read a bounded page from a verified geospatial layer",
        argument_hint: " <layer-id> [offset]",
    },
    SlashCommandDescriptor {
        name: "/geo-query",
        summary: "Query a geospatial layer with a typed attribute or CRS envelope predicate",
        argument_hint: " <layer-id> <predicate-json> [offset]",
    },
    SlashCommandDescriptor {
        name: "/geo-set",
        summary: "Propose a typed feature attribute change for shared review",
        argument_hint: " <feature-id> <field> <kind:value>",
    },
    SlashCommandDescriptor {
        name: "/geo-select",
        summary: "Select a feature from the loaded exact-snapshot page",
        argument_hint: " <feature-id>",
    },
    SlashCommandDescriptor {
        name: "/geo-geometry",
        summary: "Propose typed feature geometry JSON for shared review",
        argument_hint: " <feature-id> <geometry-json>",
    },
    SlashCommandDescriptor {
        name: "/spreadsheet-cell",
        summary: "Inspect one exact semantic worksheet cell",
        argument_hint: " <A1>",
    },
    SlashCommandDescriptor {
        name: "/spreadsheet-set",
        summary: "Propose a typed worksheet value candidate",
        argument_hint: " <A1> <text|number|boolean|blank> <value>",
    },
    SlashCommandDescriptor {
        name: "/spreadsheet-formula",
        summary: "Propose a worksheet formula candidate",
        argument_hint: " <A1> <formula>",
    },
    SlashCommandDescriptor {
        name: "/document",
        summary: "Read the selected semantic document outline and content",
        argument_hint: " [row-offset]",
    },
    SlashCommandDescriptor {
        name: "/document-select",
        summary: "Select a paragraph or table cell from the loaded document",
        argument_hint: " <entity-id>",
    },
    SlashCommandDescriptor {
        name: "/document-replace",
        summary: "Propose replacement text for a selected semantic paragraph",
        argument_hint: " <paragraph-id> <replacement text>",
    },
    SlashCommandDescriptor {
        name: "/document-cell-replace",
        summary: "Propose replacement text for a semantic table cell",
        argument_hint: " <cell-id> <replacement text>",
    },
    SlashCommandDescriptor {
        name: "/status",
        summary: "Show runtime status",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/help",
        summary: "List local commands",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/new",
        summary: "Start a new conversation",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/sessions",
        summary: "List saved conversations",
        argument_hint: "",
    },
    SlashCommandDescriptor {
        name: "/switch",
        summary: "Open a saved conversation by ID",
        argument_hint: " <session-id>",
    },
    SlashCommandDescriptor {
        name: "/remove-session",
        summary: "Remove an idle saved conversation by ID",
        argument_hint: " <session-id>",
    },
    SlashCommandDescriptor {
        name: "/recover",
        summary: "Inspect or map preserved legacy model selections",
        argument_hint: " [map <path> <revision> <provider> <model> [thinking]]",
    },
];

/// Finds command suggestions from the shared descriptor table.
///
/// - `prefix`: Slash-prefixed composer content.
///
/// Matching command descriptors in registry order.
/// # Returns
/// The operation result.
#[must_use]
pub fn suggestions(prefix: &str) -> Vec<&'static SlashCommandDescriptor> {
    if !prefix.starts_with('/') || prefix.starts_with("//") || prefix.contains(' ') {
        return Vec::new();
    }
    COMMANDS
        .iter()
        .filter(|command| command.name.starts_with(prefix))
        .collect()
}

/// Finds a descriptor for an exact command name.
///
/// - `name`: Canonical slash command spelling.
///
/// The registered descriptor when the command is recognized.
/// # Returns
/// The operation result.
#[must_use]
pub fn find(name: &str) -> Option<&'static SlashCommandDescriptor> {
    COMMANDS.iter().find(|command| command.name == name)
}

#[cfg(test)]
mod tests {
    use super::{SlashCommandDescriptor, suggestions};

    /// Keeps completion examples tied to the same registry used by parsing.
    #[test]
    fn command_prefixes_suggest_model_and_plan() {
        assert!(suggestions("/mo").iter().any(|item| item.name == "/model"));
        assert!(suggestions("/pl").iter().any(|item| item.name == "/plan"));
        assert_eq!(suggestions("//pl"), [] as [&SlashCommandDescriptor; 0]);
    }
}

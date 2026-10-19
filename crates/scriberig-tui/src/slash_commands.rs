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
        name: "/view",
        summary: "Change activity presentation",
        argument_hint: " <kind>",
    },
    SlashCommandDescriptor {
        name: "/plan",
        summary: "Create or control a durable plan",
        argument_hint: " <goal|action>",
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
        argument_hint: " [entity]",
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

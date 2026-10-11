//! Builds help content from the active keymap metadata.

use crate::keymap::{bindings, key_label, metadata};

/// One generated help row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpRow {
    /// Key label.
    pub key: String,
    /// Action label.
    pub action: String,
}

/// Generates help rows from actual bindings, without a duplicate table.
#[must_use]
pub fn rows() -> Vec<HelpRow> {
    bindings()
        .iter()
        .filter_map(|binding| {
            metadata()
                .iter()
                .find(|item| item.id == binding.action)
                .map(|item| HelpRow {
                    key: key_label(binding.chord).to_owned(),
                    action: item.label.to_owned(),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::rows;

    /// Verifies generated help uses readable labels from the active keymap.
    #[test]
    fn rows_include_readable_bound_keys() {
        let rows = rows();
        assert!(rows.iter().any(|row| row.key == "Ctrl-C"));
        assert!(rows.iter().any(|row| row.key == "Down"));
    }
}

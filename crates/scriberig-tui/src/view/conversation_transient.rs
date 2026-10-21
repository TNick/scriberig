//! Projects streamed assistant output into stable conversation rows.

use super::{ConversationModel, ConversationRow, ConversationRowKind, ConversationSource};
use scriberig_core::EntityId;

impl ConversationModel {
    /// Inserts one transient status row below its owning branch subtree.
    ///
    /// - `branch`: Stable branch identity owning the observation.
    /// - `key`: Stable client presentation identity for the operation activity.
    /// - `label`: Bounded safe status label.
    pub fn insert_transient_activity(&mut self, branch: EntityId, key: u64, label: String) {
        self.insert_transient_row(branch, key, label, ConversationRowKind::Activity);
    }

    /// Inserts streamed assistant output beneath its stable branch location.
    ///
    /// - `branch`: Durable branch owning this output.
    /// - `operation`: Stable shared identity for the output.
    /// - `text`: Safe output text rendered as a message.
    pub fn insert_transient_output(
        &mut self,
        branch: EntityId,
        operation: &scriberig_core::OperationId,
        text: &str,
    ) {
        let source = transient_output_source(branch, operation);
        self.insert_transient_row_with_source(
            branch,
            source,
            format!("Assistant: {text}"),
            ConversationRowKind::Message,
        );
    }

    /// Inserts one transient row immediately after its owning branch subtree.
    ///
    /// - `branch`: Durable branch owning the transient row.
    /// - `key`: Stable identity for this presentation observation.
    /// - `label`: Safe text for the row.
    /// - `kind`: Presentation category controlling safe text formatting.
    fn insert_transient_row(
        &mut self,
        branch: EntityId,
        key: u64,
        label: String,
        kind: ConversationRowKind,
    ) {
        let mut identity = [0; 16];
        identity[..8].copy_from_slice(&key.to_le_bytes());
        identity[8..].copy_from_slice(&branch.as_bytes()[..8]);
        self.insert_transient_row_with_source(
            branch,
            ConversationSource::TransientActivity(EntityId::from_bytes(identity)),
            label,
            kind,
        );
    }

    /// Inserts one transient row immediately after its owning branch subtree.
    ///
    /// - `branch`: Durable branch owning the transient row.
    /// - `source`: Stable transient presentation identity.
    /// - `label`: Safe text for the row.
    /// - `kind`: Presentation category controlling safe text formatting.
    fn insert_transient_row_with_source(
        &mut self,
        branch: EntityId,
        source: ConversationSource,
        label: String,
        kind: ConversationRowKind,
    ) {
        if !self
            .rows
            .iter()
            .any(|row| row.source == ConversationSource::Branch(branch))
        {
            self.rows.push(ConversationRow {
                source: ConversationSource::Branch(branch),
                reply_anchor: Some(scriberig_session::ContinuationAnchor::BranchHead { branch }),
                depth: 0,
                label: "Active branch".to_owned(),
                kind: ConversationRowKind::Branch,
            });
        }
        let Some(branch_index) = self
            .rows
            .iter()
            .position(|row| row.source == ConversationSource::Branch(branch))
        else {
            return;
        };
        self.rows.retain(|row| row.source != source);
        let depth = self.rows[branch_index].depth;
        let insertion = self.rows[branch_index + 1..]
            .iter()
            .position(|row| row.depth <= depth)
            .map_or(self.rows.len(), |offset| branch_index + 1 + offset);
        self.rows.insert(
            insertion,
            ConversationRow {
                source,
                reply_anchor: None,
                depth: depth.saturating_add(1),
                label,
                kind,
            },
        );
    }
}

/// Derives a stable UI row identity from a branch and operation identity.
///
/// - `branch`: Durable structural parent for the output.
/// - `operation`: Shared identity for the output stream.
///
/// A deterministic presentation identity independent of row position.
/// # Returns
/// The stable transient presentation source.
fn transient_output_source(
    branch: EntityId,
    operation: &scriberig_core::OperationId,
) -> ConversationSource {
    let mut first = 0xcbf2_9ce4_8422_2325u64;
    let mut second = 0x8422_2325_cbf2_9ce4u64;
    for byte in operation
        .as_str()
        .bytes()
        .chain(branch.as_bytes().iter().copied())
    {
        first = (first ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        second = (second ^ u64::from(byte))
            .rotate_left(11)
            .wrapping_mul(0x9e37_79b1_85eb_ca87);
    }
    let mut identity = [0; 16];
    identity[..8].copy_from_slice(&first.to_le_bytes());
    identity[8..].copy_from_slice(&second.to_le_bytes());
    ConversationSource::TransientActivity(EntityId::from_bytes(identity))
}

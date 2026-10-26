//! Hashes provider-visible image blocks without retaining image data.

use rpi_ai::Context;

/// Creates ordered digests for all images in the provider context.
///
/// Each digest binds the image media type, encoded content, message index,
/// and content-block index. The encoded image data is never returned.
///
/// # Parameters
/// - `provider_context`: Exact provider context being projected.
///
/// # Returns
/// Ordered, non-reversible image digests suitable for request identity.
pub(super) fn image_fingerprints(provider_context: &Context) -> Vec<String> {
    let mut fingerprints = Vec::new();
    for (message_index, message) in provider_context.messages.iter().enumerate() {
        let (role, message_content): (&str, &[rpi_ai::types::Content]) = match message {
            rpi_ai::types::Message::User(message) => match &message.content {
                rpi_ai::types::UserContent::Text(_) => continue,
                rpi_ai::types::UserContent::Blocks(blocks) => ("user", blocks),
            },
            rpi_ai::types::Message::Assistant(message) => ("assistant", &message.content),
            rpi_ai::types::Message::ToolResult(message) => ("tool", &message.content),
        };
        for (block_index, block) in message_content.iter().enumerate() {
            let rpi_ai::types::Content::Image(image) = block else {
                continue;
            };
            let material = serde_json::to_vec(&(
                "scriberig-provider-image-v1",
                role,
                message_index,
                block_index,
                &image.mime_type,
                &image.data,
            ))
            .expect("image fingerprint inputs are serializable");
            fingerprints.push(blake3::hash(&material).to_hex().to_string());
        }
    }
    fingerprints
}

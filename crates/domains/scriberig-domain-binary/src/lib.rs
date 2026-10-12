//! Preserves binary project artifacts as typed metadata documents.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Metadata for one binary source artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryDocument {
    /// Project-relative source path.
    pub path: String,
    /// Stable media-type label inferred from the path.
    pub media_type: String,
    /// Exact number of source bytes.
    pub byte_length: usize,
}

/// Creates binary metadata without decoding or modifying source bytes.
///
/// # Parameters
/// - `path`: Source path used for media-type inference.
/// - `source`: Exact source bytes.
///
/// # Returns
/// Binary metadata retaining path, type, and byte length.
#[must_use]
pub fn parse(path: &Path, source: &[u8]) -> BinaryDocument {
    BinaryDocument {
        path: path.to_string_lossy().replace('\\', "/"),
        media_type: media_type(path),
        byte_length: source.len(),
    }
}

/// Infers a stable media type for a binary path.
///
/// # Parameters
/// - `path`: Source path.
///
/// # Returns
/// Media type suitable for indexing and agent context.
fn media_type(path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let media = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "7z" => "application/x-7z-compressed",
        "jar" => "application/java-archive",
        "pdf" => "application/pdf",
        "exe" | "dll" => "application/vnd.microsoft.portable-executable",
        "o" | "a" | "so" => "application/x-object",
        _ => "application/octet-stream",
    };
    media.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves binary size and infers an image media type.
    #[test]
    fn describes_binary_artifact() {
        let document = parse(Path::new("assets/logo.png"), &[0, 1, 2]);
        assert_eq!(document.media_type, "image/png");
        assert_eq!(document.byte_length, 3);
    }
}

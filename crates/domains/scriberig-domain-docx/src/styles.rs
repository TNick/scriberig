//! Imports named DOCX styles as typed generic document extensions.

use std::io::Read;

use scriberig_domain_document::DocumentStyle;

/// Maximum uncompressed styles XML accepted by the bounded importer.
const MAX_STYLES_XML_BYTES: u64 = 64 * 1024 * 1024;

/// Reads named style identities and display names from the optional styles part.
///
/// The style definitions remain adapter-owned records. Paragraph and run
/// references continue to point to their authored style keys independently.
///
/// # Parameters
/// - `package`: Open DOCX archive containing an optional `word/styles.xml`.
///
/// # Returns
/// Style definitions in package document order.
///
/// # Errors
/// Returns an error when the styles part exceeds the size limit or contains
/// invalid XML.
pub(crate) fn read_styles<R: Read + std::io::Seek>(
    package: &mut zip::ZipArchive<R>,
) -> Result<Vec<DocumentStyle>, String> {
    let Ok(mut styles) = package.by_name("word/styles.xml") else {
        return Ok(Vec::new());
    };
    if styles.size() > MAX_STYLES_XML_BYTES {
        return Err("DOCX styles part exceeds the 64 MiB import limit".to_owned());
    }
    let mut xml = String::new();
    styles
        .read_to_string(&mut xml)
        .map_err(|error| format!("cannot read DOCX styles part: {error}"))?;
    drop(styles);
    let parsed = roxmltree::Document::parse(&xml)
        .map_err(|error| format!("DOCX styles XML is invalid: {error}"))?;
    Ok(parsed
        .root_element()
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "style")
        .filter_map(|style| {
            let key = style.attribute((
                "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                "styleId",
            ))?;
            let name = style
                .children()
                .find(|child| child.is_element() && child.tag_name().name() == "name")
                .and_then(|name| {
                    name.attribute((
                        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                        "val",
                    ))
                })
                .map(str::to_owned);
            Some(DocumentStyle {
                key: key.to_owned(),
                name,
            })
        })
        .collect())
}

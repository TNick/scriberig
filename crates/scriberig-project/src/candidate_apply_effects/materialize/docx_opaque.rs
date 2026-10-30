//! Extracts exact unchanged DOCX package payloads for shared materialization.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use sr_projection_fs::{OpaqueContentOutcome, preserve_embedded_opaque_record};
use zip::ZipArchive;

use super::super::{EffectPlan, candidate_error};
use crate::ProjectError;

/// Builds preservation receipts for unchanged members of edited DOCX packages.
///
/// - `plans`: Exact preflighted candidate effects.
///
/// # Returns
/// Embedded opaque records for every unchanged non-main-document member.
///
/// # Errors
/// Returns a candidate error if a DOCX container is invalid or its archive
/// members are ambiguous.
pub(super) fn preserved_records(
    plans: &[EffectPlan<'_>],
) -> Result<Vec<OpaqueContentOutcome>, ProjectError> {
    let mut records = Vec::new();
    for plan in plans {
        let effect = plan.effect;
        if !effect.path.to_ascii_lowercase().ends_with(".docx") {
            continue;
        }
        let (Some(source), Some(output)) = (
            effect.observed.as_deref().or(effect.old.as_deref()),
            effect.new.as_deref(),
        ) else {
            continue;
        };
        let source_parts = package_parts(source)?;
        let output_parts = package_parts(output)?;
        if source_parts.members.keys().ne(output_parts.members.keys()) {
            return Err(candidate_error(
                "docx_opaque_member_set_changed",
                &effect.path,
            ));
        }
        for (name, source_payload) in &source_parts.members {
            if name == "word/document.xml" {
                continue;
            }
            let output_payload = output_parts.members.get(name).ok_or_else(|| {
                candidate_error(
                    "docx_opaque_member_missing",
                    &format!("{}: {name}", effect.path),
                )
            })?;
            if source_payload != output_payload {
                return Err(candidate_error(
                    "docx_opaque_member_changed",
                    &format!("{}: {name}", effect.path),
                ));
            }
            let record = preserve_embedded_opaque_record(
                format!("docx-part:{name}"),
                effect.path.clone(),
                source,
                source_payload,
                output,
                output_payload,
            )
            .map_err(|error| {
                candidate_error(
                    "docx_opaque_evidence_invalid",
                    &format!("{}: {error:?}", effect.path),
                )
            })?;
            records.push(OpaqueContentOutcome::Preserved(record));
        }
        if source_parts.comment != output_parts.comment {
            return Err(candidate_error(
                "docx_opaque_archive_comment_changed",
                &effect.path,
            ));
        }
        if !source_parts.comment.is_empty() {
            let record = preserve_embedded_opaque_record(
                "docx-archive-comment",
                effect.path.clone(),
                source,
                &source_parts.comment,
                output,
                &output_parts.comment,
            )
            .map_err(|error| {
                candidate_error(
                    "docx_opaque_evidence_invalid",
                    &format!("{}: {error:?}", effect.path),
                )
            })?;
            records.push(OpaqueContentOutcome::Preserved(record));
        }
    }
    Ok(records)
}

/// Reads all unique non-directory members and the archive comment.
///
/// - `bytes`: Exact DOCX package bytes.
///
/// # Returns
/// A deterministic name-to-content map and exact archive comment.
///
/// # Errors
/// Returns a candidate error for malformed archives, duplicate names, or read
/// failures.
fn package_parts(bytes: &[u8]) -> Result<PackageParts, ProjectError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| candidate_error("docx_opaque_archive_invalid", &error.to_string()))?;
    let comment = archive.comment().to_vec();
    let mut members = BTreeMap::new();
    for index in 0..archive.len() {
        let mut member = archive
            .by_index(index)
            .map_err(|error| candidate_error("docx_opaque_member_invalid", &error.to_string()))?;
        if member.is_dir() {
            continue;
        }
        let name = member.name().to_owned();
        let mut payload = Vec::new();
        member.read_to_end(&mut payload).map_err(|error| {
            candidate_error(
                "docx_opaque_member_read_failed",
                &format!("{name}: {error}"),
            )
        })?;
        if members.insert(name.clone(), payload).is_some() {
            return Err(candidate_error("docx_opaque_duplicate_member", &name));
        }
    }
    if !members.contains_key("word/document.xml") {
        return Err(candidate_error(
            "docx_opaque_main_document_missing",
            "word/document.xml is absent",
        ));
    }
    Ok(PackageParts { members, comment })
}

/// Holds exact extracted package members and archive comment.
struct PackageParts {
    /// Unique uncompressed contents by package member name.
    members: BTreeMap<String, Vec<u8>>,
    /// Exact ZIP archive comment bytes.
    comment: Vec<u8>,
}

#[cfg(test)]
mod tests {
    //! Checks DOCX member extraction becomes source-bound materialization evidence.

    use std::io::{Cursor, Write};

    use sr_projection_fs::{OpaqueContentOutcome, digest_bytes};
    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::super::super::EffectPlan;
    use super::preserved_records;
    use crate::{ProjectSourceEffect, ProjectSourceEffectKind};

    /// Builds a minimal DOCX package with an opaque header member.
    ///
    /// - `document`: Main document XML payload.
    /// - `header`: Opaque header XML payload.
    ///
    /// # Returns
    /// A complete in-memory ZIP package.
    fn package(document: &[u8], header: &[u8]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.set_raw_comment(b"archive note".to_vec().into_boxed_slice());
        let options = SimpleFileOptions::default();
        writer
            .start_file("word/document.xml", options)
            .expect("document");
        writer.write_all(document).expect("document bytes");
        writer
            .start_file("word/header1.xml", options)
            .expect("header");
        writer.write_all(header).expect("header bytes");
        writer.finish().expect("finish package").into_inner()
    }

    /// Carries exact untouched DOCX members and archive comments into materialization.
    #[test]
    fn unchanged_parts_become_embedded_opaque_outcomes() {
        let old = package(b"<document>before</document>", b"<header>opaque</header>");
        let new = package(b"<document>after</document>", b"<header>opaque</header>");
        let effect = ProjectSourceEffect {
            path: "contract.docx".into(),
            kind: ProjectSourceEffectKind::Update,
            old: Some(old.clone()),
            observed: None,
            new: Some(new.clone()),
        };
        let plan = EffectPlan { effect: &effect };

        let records = preserved_records(&[plan]).expect("extract preserved package parts");
        assert_eq!(records.len(), 2);
        let OpaqueContentOutcome::Preserved(header) = &records[0] else {
            panic!("header outcome should be preserved");
        };
        assert_eq!(header.key, "docx-part:word/header1.xml");
        assert_eq!(header.source_member_key, "contract.docx");
        assert_eq!(header.source_member_digest, digest_bytes(&old));
        assert_eq!(header.output_container_digest, Some(digest_bytes(&new)));
        assert!(matches!(records[1], OpaqueContentOutcome::Preserved(_)));
    }

    /// Rejects a candidate that changes a package part outside the supported edit target.
    #[test]
    fn changed_untargeted_part_blocks_docx_materialization() {
        let old = package(b"<document>before</document>", b"<header>opaque</header>");
        let new = package(b"<document>after</document>", b"<header>changed</header>");
        let effect = ProjectSourceEffect {
            path: "contract.docx".into(),
            kind: ProjectSourceEffectKind::Update,
            old: Some(old),
            observed: None,
            new: Some(new),
        };
        let plan = EffectPlan { effect: &effect };

        let error = preserved_records(&[plan]).expect_err("reject unrelated package mutation");
        assert_eq!(error.code, "docx_opaque_member_changed");
    }
}

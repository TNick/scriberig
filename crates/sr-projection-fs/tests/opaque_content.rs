//! Verifies exact opaque payload carry-through and explicit discard evidence.

use sr_projection_fs::{
    OpaqueContentError, OpaqueContentOutcome, discard_opaque_record,
    preserve_embedded_opaque_record, preserve_opaque_record, validate_opaque_preservation,
};

/// Allows an understood record to change while an opaque record stays exact.
#[test]
fn semantic_change_preserves_source_bound_opaque_payload() {
    let source_member = b"known=old\nopaque=vendor-extension\n";
    let opaque = b"opaque=vendor-extension\n";
    let record = preserve_opaque_record("vendor-extension", "document.xml", source_member, opaque)
        .expect("capture source-bound opaque record");
    let output_member = b"known=new\nopaque=vendor-extension\n";

    validate_opaque_preservation(&record, &output_member[b"known=new\n".len()..])
        .expect("preserve opaque bytes");

    assert_eq!(
        record.source_member_digest,
        sr_projection_fs::digest_bytes(source_member)
    );
    assert_eq!(output_member, b"known=new\nopaque=vendor-extension\n");
}

/// Binds an exact nested package payload to both source and output containers.
#[test]
fn embedded_opaque_record_requires_exact_payload_and_binds_output_container() {
    let source = b"source compressed package";
    let output = b"rewritten compressed package";
    let payload = b"unchanged nested member";
    let record = preserve_embedded_opaque_record(
        "docx-part:word/header1.xml",
        "contract.docx",
        source,
        payload,
        output,
        payload,
    )
    .expect("capture embedded payload evidence");

    assert_eq!(record.source_member_key, "contract.docx");
    assert_eq!(
        record.source_member_digest,
        sr_projection_fs::digest_bytes(source)
    );
    assert_eq!(
        record.output_container_digest,
        Some(sr_projection_fs::digest_bytes(output))
    );
    assert_eq!(
        preserve_embedded_opaque_record(
            "docx-part:word/header1.xml",
            "contract.docx",
            source,
            payload,
            output,
            b"changed nested member",
        ),
        Err(OpaqueContentError::PayloadChanged)
    );
}

/// Rejects changed opaque bytes rather than implying preservation.
#[test]
fn modified_opaque_payload_fails_preservation_validation() {
    let record = preserve_opaque_record("extension", "document.xml", b"source", b"opaque")
        .expect("capture record");

    assert_eq!(
        validate_opaque_preservation(&record, b"rewritten"),
        Err(OpaqueContentError::PayloadChanged)
    );
}

/// Requires provenance before an operation can explicitly discard opaque data.
#[test]
fn opaque_discard_requires_and_records_provenance() {
    assert_eq!(
        discard_opaque_record(
            "extension",
            "document.xml",
            sr_projection_fs::digest_bytes(b"source"),
            ""
        ),
        Err(OpaqueContentError::MissingDiscardProvenance)
    );
    assert_eq!(
        discard_opaque_record(
            "extension",
            "document.xml",
            sr_projection_fs::digest_bytes(b"source"),
            "delete owning object",
        )
        .expect("explicit discard"),
        OpaqueContentOutcome::Discarded {
            key: "extension".into(),
            source_member_key: "document.xml".into(),
            source_member_digest: sr_projection_fs::digest_bytes(b"source"),
            provenance: "delete owning object".into(),
        }
    );
}

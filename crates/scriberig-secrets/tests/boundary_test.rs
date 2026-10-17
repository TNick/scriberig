//! Exercises secret input and destination authorization boundaries.

use scriberig_secrets::{SecretRef, SecretValue, normalize_endpoint};

/// Rejects secrets that could escape an HTTP authorization header.
#[test]
fn rejects_empty_and_multiline_values() {
    assert!(SecretValue::new(String::new()).is_err());
    assert!(SecretValue::new("key\r\nInjected: header".to_owned()).is_err());
    let value = SecretValue::new("canary-secret".to_owned()).expect("valid secret");
    assert!(!format!("{value:?}").contains("canary-secret"));
}

/// Rejects insecure endpoints and normalizes permitted numeric loopback URLs.
#[test]
fn validates_destination_before_use() {
    assert!(normalize_endpoint("http://example.com/v1").is_err());
    assert!(normalize_endpoint("https://user:password@example.com/v1").is_err());
    assert!(normalize_endpoint("https://example.com/v1?api_key=secret").is_err());
    assert_eq!(
        normalize_endpoint("http://127.0.0.1:9000/v1/").expect("local"),
        "http://127.0.0.1:9000/v1"
    );
}

/// Keeps reference representations typed and rejects inline secret material.
#[test]
fn rejects_inline_credential_objects() {
    assert!(
        serde_json::from_str::<SecretRef>(r#"{"source":"keyring","name":"work","value":"secret"}"#)
            .is_err()
    );
}

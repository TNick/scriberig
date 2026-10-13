//! Protects the absent-PHP baseline required by the self-hosting milestone.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// PHP-GOLDEN-FIXTURE-01 validates fixture ownership against the checkout.
#[test]
fn php_fixture_effect_manifest_matches_existing_repository_owners() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/self-hosting/php-extension/manifest.json"
    ))
    .expect("fixture manifest");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = manifest["candidate_paths"]
        .as_array()
        .expect("candidate paths");
    assert_ne!(paths.len(), 0);
    for effect in paths {
        let path = effect["path"].as_str().expect("relative candidate path");
        assert!(!Path::new(path).is_absolute());
        assert!(!path.split('/').any(|part| part == ".."));
        let target = repository.join(path);
        match effect["kind"].as_str() {
            Some("create") => assert!(!target.exists(), "create path already exists: {path}"),
            Some("update") => assert!(target.is_file(), "update owner is missing: {path}"),
            other => panic!("unexpected candidate effect kind: {other:?}"),
        }
    }
    assert!(
        repository
            .join("tests/self-hosting/php-extension/php-project/src/Greeter.php")
            .is_file()
    );
}

/// PHP-GOLDEN-BASELINE-01 proves PHP is not a first-class baseline adapter.
#[test]
fn php_is_generic_text_before_self_hosting_candidate_apply() {
    let root = std::env::temp_dir().join(format!("scriberig-php-baseline-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    fs::write(root.join("Example.php"), "<?php class Example {}\n").expect("PHP source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(database).expect("database"));
    let report = ProjectService::new(store)
        .scan(Some("php-baseline".to_owned()), &root)
        .expect("scan baseline");
    let file = report
        .files
        .iter()
        .find(|file| file.relative_path == "Example.php")
        .expect("PHP file in manifest");
    assert_eq!(file.adapter, "text");
    assert_eq!(file.status, "ready");
    assert!(!root.join("crates/domains/scriberig-domain-php").exists());
}

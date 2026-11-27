//! The app version must be the same in Cargo.toml, tauri.conf.json and package.json, and the
//! changelog must have an entry for it.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap

use std::fs;
use std::path::Path;

fn json_version(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["version"].as_str().unwrap().to_owned()
}

#[test]
fn versions_match_and_are_in_the_changelog() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        json_version(&manifest_dir.join("tauri.conf.json")),
        cargo_version,
        "tauri.conf.json"
    );
    assert_eq!(
        json_version(&manifest_dir.join("../package.json")),
        cargo_version,
        "apps/desktop/package.json"
    );

    let changelog = fs::read_to_string(manifest_dir.join("../../../CHANGELOG.md")).unwrap();
    assert!(
        changelog.contains("## [Unreleased]"),
        "CHANGELOG.md needs an [Unreleased] section"
    );
    assert!(
        changelog.contains(&format!("## [{cargo_version}]")),
        "CHANGELOG.md has no entry for version {cargo_version}"
    );
}

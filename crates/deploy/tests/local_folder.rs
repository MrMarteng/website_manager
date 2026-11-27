//! End-to-end test with the real rclone against a local folder standing in for the server.
//! Skipped when rclone is not installed (set `RCLONE` to use a specific binary).
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use teleschirm_deploy::{
    Connection, DeployConfig, MappingMode, Runner, VERSION_FILE, VersionStamp,
};

fn rclone() -> Option<PathBuf> {
    let program = std::env::var_os("RCLONE").map_or_else(|| PathBuf::from("rclone"), PathBuf::from);
    Command::new(&program)
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| program)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const CONFIG: &str = "\
host = example.com
user = web1
dist/backend/data -> /site/backend/data | mode=seed
dist -> /site | protect=backend/data/*.json
";

#[test]
fn sync_respects_seed_protect_and_version_file() {
    let Some(rclone) = rclone() else {
        eprintln!("rclone not found – test skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let server = dir.path().join("server");

    write(&project.join("dist/index.html"), "neu");
    write(
        &project.join("dist/backend/data/news.json"),
        "beispieldaten",
    );
    write(&server.join("site/old.html"), "alt");
    write(&server.join("site/backend/data/news.json"), "live-daten");

    let mut config = DeployConfig::parse(CONFIG, &project).unwrap();
    config.base_dir.clone_from(&project);
    let connection = Connection::LocalFolder(server.clone());
    let stamp = VersionStamp {
        name: "beispiel".into(),
        version: "1.0.0".into(),
        created: "2026-11-27T08:00:00Z".into(),
    };

    // Dry run changes nothing but lists the changes.
    let mut log = Vec::new();
    let preview = Runner::new(rclone.clone(), true)
        .run(
            &config,
            MappingMode::Sync,
            &connection,
            Some(&stamp),
            &mut |line| log.push(line.to_owned()),
        )
        .unwrap();
    assert!(server.join("site/old.html").exists());
    assert!(!server.join("site/index.html").exists());
    assert!(
        preview.uploaded.contains(&"/site/index.html".to_owned()),
        "{:?}",
        preview.uploaded
    );
    assert_eq!(preview.deleted, vec!["/site/old.html"]);

    let runner = Runner::new(rclone, false);
    let summary = runner
        .run(
            &config,
            MappingMode::Sync,
            &connection,
            Some(&stamp),
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(summary.skipped_seed, 1, "seed target already had content");
    assert_eq!(summary.deleted, vec!["/site/old.html"]);
    assert_eq!(
        fs::read_to_string(server.join("site/index.html")).unwrap(),
        "neu"
    );
    assert!(
        !server.join("site/old.html").exists(),
        "sync removes stale files"
    );
    assert_eq!(
        fs::read_to_string(server.join("site/backend/data/news.json")).unwrap(),
        "live-daten",
        "protected CMS data must survive"
    );
    assert!(server.join("site").join(VERSION_FILE).exists());

    // A second sync must keep the version file, and it must be readable.
    runner
        .run(&config, MappingMode::Sync, &connection, None, &mut |_| {})
        .unwrap();
    assert_eq!(
        runner.read_server_version(&config, &connection).unwrap(),
        Some(stamp)
    );
}

#[test]
fn seed_fills_empty_target() {
    let Some(rclone) = rclone() else {
        eprintln!("rclone not found – test skipped");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let server = dir.path().join("server");
    write(&project.join("dist/index.html"), "neu");
    write(&project.join("dist/backend/data/news.json"), "startdaten");

    let mut config = DeployConfig::parse(CONFIG, &project).unwrap();
    config.base_dir.clone_from(&project);
    let connection = Connection::LocalFolder(server.clone());
    let runner = Runner::new(rclone, false);
    let summary = runner
        .run(&config, MappingMode::Sync, &connection, None, &mut |_| {})
        .unwrap();
    assert_eq!(summary.skipped_seed, 0);
    assert_eq!(
        fs::read_to_string(server.join("site/backend/data/news.json")).unwrap(),
        "startdaten"
    );
    assert_eq!(
        runner.read_server_version(&config, &connection).unwrap(),
        None
    );
}

//! What the window shows about a project.

use std::path::Path;

use serde::Serialize;
use teleschirm_deploy::{MappingMode, Protocol};
use wm_core::credentials::CREDENTIALS_FILE;
use wm_core::manifest::Manifest;
use wm_core::projects::{ProjectSettings, ProjectStore};
use wm_core::signing::fingerprint;
use wm_core::trust::TrustStore;

use crate::deployment::effective_config;
use crate::keychain;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub version: String,
    pub created: String,
    pub changelog: Vec<String>,
    pub current: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingInfo {
    pub source: String,
    pub target: String,
    pub mode: String,
    pub protect: Vec<String>,
}

/// Upload settings in effect (package `deploy.conf` plus the customer's settings).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detected {
    pub protocol: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub root: String,
    pub mappings: Vec<MappingInfo>,
    /// The package's recorded host key applies to the server in use.
    pub host_key_pinned: bool,
}

/// Flags of a project as the window needs them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub name: String,
    pub publisher: String,
    pub current: VersionInfo,
    pub versions: Vec<VersionInfo>,
    pub settings: ProjectSettings,
    /// `None` if the package has no `deploy.conf` (no upload possible).
    pub detected: Option<Detected>,
    pub deploy_error: Option<String>,
    pub stored_credentials: bool,
    pub package_has_credentials: bool,
    pub has_compliance: bool,
    pub has_sources: bool,
}

fn version_info(manifest: &Manifest, id: &str, current: &str) -> VersionInfo {
    VersionInfo {
        id: id.to_owned(),
        version: manifest.version.clone(),
        created: manifest.created.clone(),
        changelog: manifest.changelog.clone(),
        current: id == current,
    }
}

fn mode_name(mode: Option<MappingMode>) -> &'static str {
    match mode {
        Some(MappingMode::Seed) => "seed",
        Some(MappingMode::Copy) => "copy",
        Some(MappingMode::Sync) | None => "sync",
    }
}

fn detect(folder: &Path, settings: &ProjectSettings) -> (Option<Detected>, Option<String>) {
    if !folder.join("deploy.conf").is_file() {
        return (None, None);
    }
    match effective_config(folder, settings) {
        Ok((config, pinned)) => {
            let detected = Detected {
                protocol: match config.server.protocol {
                    Protocol::Sftp => "sftp",
                    Protocol::Ftps => "ftps",
                }
                .into(),
                host: config.server.host.clone(),
                user: config.server.user.clone(),
                port: config.server.port,
                root: teleschirm_deploy::plan::site_root(&config).unwrap_or_default(),
                mappings: config
                    .mappings
                    .iter()
                    .map(|mapping| MappingInfo {
                        source: mapping.source.clone(),
                        target: mapping.target.clone(),
                        mode: mode_name(mapping.mode).into(),
                        protect: mapping.protect.clone(),
                    })
                    .collect(),
                host_key_pinned: pinned,
            };
            (Some(detected), None)
        }
        Err(error) => (None, Some(error)),
    }
}

/// Collects the overview of a project.
///
/// # Errors
/// Fails if the project or its current version cannot be read.
pub fn build(store: &ProjectStore, trust_file: &Path, name: &str) -> Result<Overview, String> {
    let record = store.load(name).map_err(|e| e.to_string())?;
    let manifest = store
        .manifest(name, &record.current)
        .map_err(|e| e.to_string())?;
    let folder = store.version_dir(name, &record.current);
    let versions = record
        .versions
        .iter()
        .filter_map(|id| {
            store
                .manifest(name, id)
                .ok()
                .map(|m| version_info(&m, id, &record.current))
        })
        .collect();
    let publisher = TrustStore::load(trust_file)
        .ok()
        .and_then(|store| store.publisher(name))
        .map(|key| fingerprint(&key))
        .unwrap_or_default();
    let (detected, deploy_error) = detect(&folder, &record.settings);
    let has_prefix = |prefix: &str| manifest.files.keys().any(|path| path.starts_with(prefix));

    Ok(Overview {
        name: name.to_owned(),
        publisher,
        current: version_info(&manifest, &record.current, &record.current),
        versions,
        settings: record.settings.clone(),
        detected,
        deploy_error,
        stored_credentials: keychain::has(name),
        package_has_credentials: manifest.has_credentials
            && folder.join(CREDENTIALS_FILE).is_file(),
        has_compliance: has_prefix("compliance/"),
        has_sources: has_prefix("project/"),
    })
}

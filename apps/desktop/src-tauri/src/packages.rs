//! Opening packages: verification, publisher pinning and extraction into the app data folder.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use wm_core::archive::{extract_package, verify_package};
use wm_core::manifest::Manifest;
use wm_core::signing::fingerprint;
use wm_core::trust::{TrustDecision, TrustStore};

/// Copy of the manifest kept next to the extracted files.
pub const MANIFEST_COPY: &str = ".website-manager-manifest.json";

/// What the window shows about an opened package (flat data for the window).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub created: String,
    pub publisher: String,
    pub first_use: bool,
    pub changelog: Vec<String>,
    pub file_count: usize,
    pub has_sources: bool,
    pub has_deploy_conf: bool,
    pub has_compliance_report: bool,
    pub folder: String,
}

/// Verifies a package, checks the publisher and extracts it below `projects_dir`.
///
/// An already extracted package with the same name, version and creation time is reused.
///
/// # Errors
/// Fails if the package is invalid, the publisher does not match, or extraction fails.
pub fn open(
    package: &Path,
    projects_dir: &Path,
    trust_file: &Path,
) -> Result<PackageSummary, String> {
    let verified = verify_package(package).map_err(|e| e.to_string())?;
    let manifest = &verified.manifest;
    let mut trust = TrustStore::load(trust_file).map_err(|e| e.to_string())?;
    let decision = trust
        .check(&manifest.name, &verified.publisher)
        .map_err(|e| e.to_string())?;

    let folder = projects_dir.join(&manifest.name).join(format!(
        "{}_{}",
        manifest.version,
        manifest.created.replace([':', '-'], "")
    ));
    if !folder.exists() {
        if let Some(parent) = folder.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        extract_package(package, &folder).map_err(|e| e.to_string())?;
        let bytes = manifest.to_canonical_bytes().map_err(|e| e.to_string())?;
        fs::write(folder.join(MANIFEST_COPY), bytes).map_err(|e| e.to_string())?;
    }
    if decision == TrustDecision::FirstUse {
        trust
            .bind(&manifest.name, &verified.publisher)
            .map_err(|e| e.to_string())?;
    }
    Ok(summary(
        manifest,
        &fingerprint(&verified.publisher),
        decision,
        &folder,
    ))
}

/// Reads the manifest copy of an extracted package.
///
/// # Errors
/// Fails if the folder is not an extracted package.
pub fn load_manifest(folder: &Path) -> Result<Manifest, String> {
    let bytes = fs::read(folder.join(MANIFEST_COPY)).map_err(|e| e.to_string())?;
    Manifest::from_bytes(&bytes).map_err(|e| e.to_string())
}

/// Makes sure `folder` is a package folder below `projects_dir` (the window passes it back).
///
/// # Errors
/// Fails for folders outside the app's project folder.
pub fn checked_folder(folder: &str, projects_dir: &Path) -> Result<PathBuf, String> {
    let folder = fs::canonicalize(folder).map_err(|e| e.to_string())?;
    let root = fs::canonicalize(projects_dir).map_err(|e| e.to_string())?;
    if folder.starts_with(&root) && folder.join(MANIFEST_COPY).is_file() {
        Ok(folder)
    } else {
        Err("folder is not an opened package".into())
    }
}

fn summary(
    manifest: &Manifest,
    publisher: &str,
    decision: TrustDecision,
    folder: &Path,
) -> PackageSummary {
    let has_prefix = |prefix: &str| manifest.files.keys().any(|path| path.starts_with(prefix));
    PackageSummary {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        created: manifest.created.clone(),
        publisher: publisher.to_owned(),
        first_use: decision == TrustDecision::FirstUse,
        changelog: manifest.changelog.clone(),
        file_count: manifest.files.len(),
        has_sources: has_prefix("project/"),
        has_deploy_conf: manifest.files.contains_key("deploy.conf"),
        has_compliance_report: has_prefix("compliance/"),
        folder: folder.to_string_lossy().into_owned(),
    }
}

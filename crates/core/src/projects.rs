//! Opened projects on the customer's computer: versions, current version and settings.
//!
//! Layout below the store root:
//!
//! ```text
//! <name>/project.json          current version, version list, settings
//! <name>/versions/<id>/        extracted package (id = <version>_<created>)
//! ```
//!
//! Settings live in `project.json`, outside the package folders, so updates keep them.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::CoreError;
use crate::archive::{VerifiedPackage, extract_package, verify_package};
use crate::hashing::sha256_file;
use crate::manifest::Manifest;
use crate::trust::{TrustDecision, TrustStore};

/// File name of the manifest copy inside an extracted version.
pub const MANIFEST_COPY: &str = ".website-manager-manifest.json";
/// Number of versions kept per project; older ones are handed back for removal.
pub const KEPT_VERSIONS: usize = 5;

/// Settings the customer can change; `None` keeps the value from the package's `deploy.conf`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Replaces the website root folder on the server in all mappings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_root: Option<String>,
    /// Public address of the website, opened after an upload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Expert mode: mapping lines that replace those of the package's `deploy.conf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mappings: Option<String>,
}

/// Content of `project.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub name: String,
    /// Id of the current version.
    pub current: String,
    /// Version ids, newest first.
    pub versions: Vec<String>,
    #[serde(default)]
    pub settings: ProjectSettings,
    /// Credentials for this project are in the OS keychain. Kept here so the app never has to
    /// read the keychain (which may prompt for a password) just to show this.
    #[serde(default)]
    pub credentials_stored: bool,
    /// Last successful upload (not a dry run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_upload: Option<LastUpload>,
}

/// What was uploaded last, and where to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastUpload {
    pub version: String,
    /// `created` of the uploaded package.
    pub created: String,
    /// Time of the upload (RFC 3339, UTC).
    pub uploaded_at: String,
    /// `server` or `folder`.
    pub target: String,
    /// Host name or folder path.
    pub destination: String,
}

/// Outcome of importing a package.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub package: VerifiedPackage,
    pub decision: TrustDecision,
    pub version_id: String,
    pub folder: PathBuf,
    /// Version that was current before, if any.
    pub previous: Option<String>,
    /// Files of the previous version that differ from what was delivered (edited by the customer).
    pub locally_changed: Vec<String>,
    /// Version folders beyond [`KEPT_VERSIONS`], to be moved to the trash by the caller.
    pub outdated: Vec<PathBuf>,
}

/// All projects below one folder.
pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn project_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn record_path(&self, name: &str) -> PathBuf {
        self.project_dir(name).join("project.json")
    }

    /// Folder of a version.
    #[must_use]
    pub fn version_dir(&self, name: &str, version_id: &str) -> PathBuf {
        self.project_dir(name).join("versions").join(version_id)
    }

    /// Reads a project record.
    ///
    /// # Errors
    /// Fails if the project does not exist or the record is unreadable.
    pub fn load(&self, name: &str) -> Result<ProjectRecord, CoreError> {
        let path = self.record_path(name);
        let text = fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
        serde_json::from_str(&text).map_err(|e| CoreError::json("project.json", e))
    }

    /// Writes a project record.
    ///
    /// # Errors
    /// Fails if the record cannot be written.
    pub fn save(&self, record: &ProjectRecord) -> Result<(), CoreError> {
        let path = self.record_path(&record.name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let text =
            serde_json::to_string_pretty(record).map_err(|e| CoreError::json("project.json", e))?;
        fs::write(&path, text).map_err(|e| CoreError::io(&path, e))
    }

    /// Names of all projects, sorted.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("project.json").is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Verifies a package, checks its publisher, extracts it and makes it the current version.
    /// Settings of an existing project are kept.
    ///
    /// # Errors
    /// Fails if the package is invalid, signed by another publisher, or cannot be extracted.
    pub fn import(&self, package: &Path, trust_file: &Path) -> Result<ImportResult, CoreError> {
        let verified = verify_package(package)?;
        let name = verified.manifest.name.clone();
        let mut trust = TrustStore::load(trust_file)?;
        let decision = trust.check(&name, &verified.publisher)?;

        let version_id = version_id(&verified.manifest);
        let folder = self.version_dir(&name, &version_id);
        if !folder.exists() {
            if let Some(parent) = folder.parent() {
                fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
            }
            extract_package(package, &folder)?;
            let bytes = verified.manifest.to_canonical_bytes()?;
            let copy = folder.join(MANIFEST_COPY);
            fs::write(&copy, bytes).map_err(|e| CoreError::io(&copy, e))?;
        }
        if decision == TrustDecision::FirstUse {
            trust.bind(&name, &verified.publisher)?;
        }

        let mut record = self.load(&name).unwrap_or_else(|_| ProjectRecord {
            name: name.clone(),
            ..ProjectRecord::default()
        });
        let previous = (!record.current.is_empty() && record.current != version_id)
            .then(|| record.current.clone());
        let locally_changed = match &previous {
            Some(id) => self.locally_changed(&name, id)?,
            None => Vec::new(),
        };

        record.versions.retain(|id| id != &version_id);
        record.versions.insert(0, version_id.clone());
        record.current.clone_from(&version_id);
        let outdated = record
            .versions
            .split_off(record.versions.len().min(KEPT_VERSIONS))
            .into_iter()
            .map(|id| self.version_dir(&name, &id))
            .collect();
        self.save(&record)?;

        Ok(ImportResult {
            package: verified,
            decision,
            version_id,
            folder,
            previous,
            locally_changed,
            outdated,
        })
    }

    /// Makes an existing version the current one (restore).
    ///
    /// # Errors
    /// Fails if the project or version does not exist.
    pub fn set_current(&self, name: &str, version_id: &str) -> Result<ProjectRecord, CoreError> {
        let mut record = self.load(name)?;
        if !record.versions.iter().any(|id| id == version_id) {
            return Err(CoreError::MissingEntry(version_id.to_owned()));
        }
        version_id.clone_into(&mut record.current);
        self.save(&record)?;
        Ok(record)
    }

    /// Removes a version from the list (not the current one) and returns its folder, which
    /// the caller moves to the trash.
    ///
    /// # Errors
    /// Fails for the current version or an unknown version.
    pub fn remove_version(&self, name: &str, version_id: &str) -> Result<PathBuf, CoreError> {
        let mut record = self.load(name)?;
        if record.current == version_id || !record.versions.iter().any(|id| id == version_id) {
            return Err(CoreError::MissingEntry(version_id.to_owned()));
        }
        record.versions.retain(|id| id != version_id);
        self.save(&record)?;
        Ok(self.version_dir(name, version_id))
    }

    /// Reads the manifest of an extracted version.
    ///
    /// # Errors
    /// Fails if the version folder is missing or invalid.
    pub fn manifest(&self, name: &str, version_id: &str) -> Result<Manifest, CoreError> {
        read_manifest_copy(&self.version_dir(name, version_id))
    }

    /// Files of a version that no longer match the delivered checksums.
    ///
    /// # Errors
    /// Fails if the manifest copy cannot be read.
    pub fn locally_changed(&self, name: &str, version_id: &str) -> Result<Vec<String>, CoreError> {
        let folder = self.version_dir(name, version_id);
        let manifest = read_manifest_copy(&folder)?;
        Ok(manifest
            .files
            .iter()
            .filter(|(path, expected)| {
                sha256_file(&folder.join(path.as_str())).ok().as_ref() != Some(*expected)
            })
            .map(|(path, _)| path.clone())
            .collect())
    }
}

/// Reads the manifest copy stored in an extracted version folder.
///
/// # Errors
/// Fails if the copy is missing or invalid.
pub fn read_manifest_copy(folder: &Path) -> Result<Manifest, CoreError> {
    let path = folder.join(MANIFEST_COPY);
    let bytes = fs::read(&path).map_err(|e| CoreError::io(&path, e))?;
    Manifest::from_bytes(&bytes)
}

/// Folder name of a version: `<version>_<created without : and ->`.
#[must_use]
pub fn version_id(manifest: &Manifest) -> String {
    format!(
        "{}_{}",
        manifest.version,
        manifest.created.replace([':', '-'], "")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FORMAT_VERSION;
    use crate::archive::{PackageEntry, write_package};
    use crate::signing::generate_signing_key;
    use ed25519_dalek::SigningKey;
    use std::collections::BTreeMap;

    fn package(dir: &Path, key: &SigningKey, version: &str, created: &str, html: &str) -> PathBuf {
        let source = dir.join(format!("index-{version}.html"));
        fs::write(&source, html).unwrap();
        let output = dir.join(format!("cafe-{version}.tsweb"));
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            name: "cafe".into(),
            version: version.into(),
            created: created.into(),
            node_version: None,
            build_output: None,
            changelog: vec![],
            logo: None,
            has_credentials: false,
            files: BTreeMap::new(),
        };
        write_package(
            &output,
            manifest,
            &[PackageEntry {
                package_path: "dist/index.html".into(),
                source,
            }],
            key,
        )
        .unwrap();
        output
    }

    #[test]
    fn update_keeps_settings_and_reports_local_changes() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::new(dir.path().join("projects"));
        let trust = dir.path().join("trust.json");
        let key = generate_signing_key().unwrap();

        let first = package(dir.path(), &key, "1.0.0", "2026-11-01T10:00:00Z", "eins");
        let imported = store.import(&first, &trust).unwrap();
        assert_eq!(imported.decision, TrustDecision::FirstUse);
        assert!(imported.previous.is_none());

        let mut record = store.load("cafe").unwrap();
        record.settings.host = Some("neu.example.com".into());
        store.save(&record).unwrap();
        fs::write(
            imported.folder.join("dist/index.html"),
            "vom Kunden geändert",
        )
        .unwrap();

        let second = package(dir.path(), &key, "1.1.0", "2026-11-20T10:00:00Z", "zwei");
        let update = store.import(&second, &trust).unwrap();
        assert_eq!(update.decision, TrustDecision::Known);
        assert_eq!(update.previous.as_deref(), Some("1.0.0_20261101T100000Z"));
        assert_eq!(update.locally_changed, vec!["dist/index.html"]);

        let record = store.load("cafe").unwrap();
        assert_eq!(record.current, "1.1.0_20261120T100000Z");
        assert_eq!(record.versions.len(), 2);
        assert_eq!(record.settings.host.as_deref(), Some("neu.example.com"));
        assert_eq!(store.list(), vec!["cafe"]);

        let restored = store.set_current("cafe", "1.0.0_20261101T100000Z").unwrap();
        assert_eq!(restored.current, "1.0.0_20261101T100000Z");
        assert!(store.set_current("cafe", "9.9.9_x").is_err());
        assert!(
            store
                .remove_version("cafe", "1.0.0_20261101T100000Z")
                .is_err(),
            "current version"
        );
        let removed = store
            .remove_version("cafe", "1.1.0_20261120T100000Z")
            .unwrap();
        assert!(removed.ends_with("versions/1.1.0_20261120T100000Z"));
        assert_eq!(store.load("cafe").unwrap().versions.len(), 1);
    }

    #[test]
    fn keeps_only_the_newest_versions() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::new(dir.path().join("projects"));
        let trust = dir.path().join("trust.json");
        let key = generate_signing_key().unwrap();
        let mut outdated = Vec::new();
        for day in 1..=KEPT_VERSIONS + 2 {
            let created = format!("2026-11-{day:02}T10:00:00Z");
            let path = package(dir.path(), &key, &format!("1.{day}.0"), &created, "x");
            outdated.extend(store.import(&path, &trust).unwrap().outdated);
        }
        assert_eq!(store.load("cafe").unwrap().versions.len(), KEPT_VERSIONS);
        assert_eq!(outdated.len(), 2);
        assert!(outdated[0].ends_with("1.1.0_20261101T100000Z"));
    }
}

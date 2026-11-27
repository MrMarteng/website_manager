//! `website-manager pack`: build a project and write a signed package.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use teleschirm_deploy::{DeployConfig, Protocol};
use walkdir::WalkDir;
use wm_core::archive::{PackageEntry, write_package};
use wm_core::credentials::{self, CREDENTIALS_FILE, Credentials};
use wm_core::manifest::Manifest;
use wm_core::secrets::SecretScanner;
use wm_core::{FORMAT_VERSION, signing};

/// File name of the project configuration.
pub const PROJECT_CONFIG: &str = "website-manager.config.json";

/// Folders that are never packed from the project sources.
const ALWAYS_SKIPPED: &[&str] = &[
    "node_modules",
    ".git",
    ".astro",
    ".venv",
    ".DS_Store",
    "Thumbs.db",
];

/// `deploy.conf` keys that point to files on the publisher's computer.
const MACHINE_SPECIFIC_KEYS: &[&str] = &["key_file", "known_hosts_file"];

/// Package path of the recorded server host keys.
const KNOWN_HOSTS_FILE: &str = "known_hosts";

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("{path}: {message}")]
    File { path: PathBuf, message: String },
    #[error("invalid {PROJECT_CONFIG}: {0}")]
    Config(String),
    #[error("build failed ({0})")]
    Build(String),
    #[error(transparent)]
    Core(#[from] wm_core::CoreError),
}

/// Content of `website-manager.config.json`.
#[derive(Debug, Deserialize)]
struct ProjectConfig {
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default = "default_build")]
    build: Vec<String>,
    #[serde(default = "default_dist")]
    dist: String,
    #[serde(default)]
    deploy_conf: Option<String>,
    #[serde(default)]
    node_version: Option<String>,
    /// Globs of project sources to ship (needed to rebuild or run the development server).
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    changelog: Vec<String>,
    /// Compliance report shipped under `compliance/`.
    #[serde(default)]
    compliance: Option<String>,
    #[serde(default)]
    logo: Option<String>,
}

fn default_build() -> Vec<String> {
    vec!["npm".into(), "run".into(), "build".into()]
}

fn default_dist() -> String {
    "dist".into()
}

pub struct PackOptions {
    pub project: PathBuf,
    pub key: PathBuf,
    pub output: Option<PathBuf>,
    pub release: Option<String>,
    pub skip_build: bool,
    /// Hoster credentials to ship encrypted (first delivery only).
    pub credentials: Option<Credentials>,
    /// Password protecting the credentials; required together with `credentials`.
    pub package_password: Option<String>,
    /// Record the server's SSH host key so the app can verify the server.
    pub host_key: bool,
}

pub struct PackReport {
    pub output: PathBuf,
    pub version: String,
    pub files: usize,
    pub warnings: Vec<String>,
}

/// Builds the project and writes the package.
///
/// # Errors
/// Fails if the configuration is invalid, the build fails, secrets are found or writing fails.
pub fn pack(options: &PackOptions) -> Result<PackReport, PackError> {
    let project = &options.project;
    let config = load_config(project)?;
    let key = signing::load_signing_key(&options.key)?;
    let version = options
        .release
        .clone()
        .or_else(|| config.version.clone())
        .ok_or_else(|| PackError::Config("version missing (config or --release)".into()))?;

    if !options.skip_build {
        run_build(project, &config.build)?;
    }

    let mut warnings = Vec::new();
    let staging_dir = tempfile::tempdir().map_err(|e| PackError::File {
        path: std::env::temp_dir(),
        message: e.to_string(),
    })?;
    let staging = staging_dir.path();
    let mut entries = collect_tree(&project.join(&config.dist), "dist", None)?;
    if entries.is_empty() {
        return Err(PackError::Config(format!(
            "build output {} is empty",
            config.dist
        )));
    }
    let include = build_globs(&config.include)?;
    let exclude = build_globs(&config.exclude)?;
    if !config.include.is_empty() {
        entries.extend(
            collect_tree(project, "project", Some((&include, &exclude)))?
                .into_iter()
                .filter(|entry| {
                    !entry
                        .package_path
                        .starts_with(&format!("project/{}/", config.dist))
                }),
        );
    }
    if let Some(deploy_conf) = &config.deploy_conf {
        add_deploy_conf(
            &project.join(deploy_conf),
            staging,
            options.host_key,
            &mut entries,
            &mut warnings,
        )?;
    }
    if let Some(report) = &config.compliance {
        let source = project.join(report);
        let name = source.file_name().map_or_else(
            || "report".into(),
            |name| name.to_string_lossy().into_owned(),
        );
        entries.push(PackageEntry {
            package_path: format!("compliance/{name}"),
            source,
        });
    }

    let has_credentials = add_credentials(options, staging, &mut entries)?;

    let scanner = SecretScanner::new()?;
    let findings: Vec<String> = entries
        .iter()
        .filter_map(|entry| scanner.check(&entry.package_path, &entry.source))
        .collect();
    if !findings.is_empty() {
        return Err(wm_core::CoreError::SecretsFound(findings).into());
    }

    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        name: config.name.clone(),
        version: version.clone(),
        created: crate::time::now_rfc3339(),
        node_version: config.node_version.clone(),
        changelog: config.changelog.clone(),
        logo: config.logo.clone().map(|logo| format!("project/{logo}")),
        has_credentials,
        files: std::collections::BTreeMap::new(),
    };
    let output = options
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{}-{version}.tsweb", config.name)));
    let manifest = write_package(&output, manifest, &entries, &key)?;

    Ok(PackReport {
        output,
        version,
        files: manifest.files.len(),
        warnings,
    })
}

/// Adds the sanitized `deploy.conf` and, if wanted and reachable, the server's host keys.
fn add_deploy_conf(
    source: &Path,
    staging: &Path,
    host_key: bool,
    entries: &mut Vec<PackageEntry>,
    warnings: &mut Vec<String>,
) -> Result<(), PackError> {
    let known_hosts_line = if host_key {
        match scan_host_key(source) {
            Ok(Some(keys)) => {
                let known_hosts = staging.join(KNOWN_HOSTS_FILE);
                fs::write(&known_hosts, keys).map_err(|e| PackError::File {
                    path: known_hosts.clone(),
                    message: e.to_string(),
                })?;
                entries.push(PackageEntry {
                    package_path: KNOWN_HOSTS_FILE.into(),
                    source: known_hosts,
                });
                Some(format!("known_hosts_file = {KNOWN_HOSTS_FILE}"))
            }
            Ok(None) => None,
            Err(message) => {
                warnings.push(format!(
                    "Host-Schlüssel nicht ermittelt ({message}) – der Kunde muss den Server beim ersten Upload bestätigen"
                ));
                None
            }
        }
    } else {
        None
    };
    let sanitized = staging.join("deploy.conf");
    let removed = sanitize_deploy_conf(source, &sanitized, known_hosts_line.as_deref())?;
    if !removed.is_empty() {
        warnings.push(format!(
            "deploy.conf: {} entfernt – Zugang richtet der Kunde im Website Manager ein",
            removed.join(", ")
        ));
    }
    entries.push(PackageEntry {
        package_path: "deploy.conf".into(),
        source: sanitized,
    });
    Ok(())
}

/// Adds `credentials.enc` if credentials were given. Returns whether the package has them.
fn add_credentials(
    options: &PackOptions,
    staging: &Path,
    entries: &mut Vec<PackageEntry>,
) -> Result<bool, PackError> {
    match (&options.credentials, &options.package_password) {
        (None, _) => Ok(false),
        (Some(credentials), Some(password)) => {
            let encrypted = staging.join(CREDENTIALS_FILE);
            let content = credentials::encrypt(credentials, password)?;
            fs::write(&encrypted, content).map_err(|e| PackError::File {
                path: encrypted.clone(),
                message: e.to_string(),
            })?;
            entries.push(PackageEntry {
                package_path: CREDENTIALS_FILE.into(),
                source: encrypted,
            });
            Ok(true)
        }
        (Some(_), None) => Err(PackError::Config(
            "package password missing for credentials".into(),
        )),
    }
}

fn load_config(project: &Path) -> Result<ProjectConfig, PackError> {
    let path = project.join(PROJECT_CONFIG);
    let text = fs::read_to_string(&path).map_err(|e| PackError::File {
        path: path.clone(),
        message: e.to_string(),
    })?;
    let config: ProjectConfig =
        serde_json::from_str(&text).map_err(|e| PackError::Config(e.to_string()))?;
    if config.name.is_empty()
        || !config
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PackError::Config(
            "name must consist of a-z, 0-9, - and _".into(),
        ));
    }
    Ok(config)
}

fn run_build(project: &Path, command: &[String]) -> Result<(), PackError> {
    let Some((program, args)) = command.split_first() else {
        return Ok(());
    };
    let status = Command::new(program)
        .args(args)
        .current_dir(project)
        .status()
        .map_err(|e| PackError::Build(format!("{program}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(PackError::Build(status.to_string()))
    }
}

fn build_globs(patterns: &[String]) -> Result<GlobSet, PackError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).map_err(|e| PackError::Config(format!("{pattern}: {e}")))?);
    }
    builder
        .build()
        .map_err(|e| PackError::Config(e.to_string()))
}

/// Collects regular files below `root`, stored under `prefix/`. With `filter`, only paths
/// matching the include set and not the exclude set are taken (positive list).
fn collect_tree(
    root: &Path,
    prefix: &str,
    filter: Option<(&GlobSet, &GlobSet)>,
) -> Result<Vec<PackageEntry>, PackError> {
    let mut entries = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            !ALWAYS_SKIPPED
                .iter()
                .any(|skipped| entry.file_name() == *skipped)
        });
    for entry in walker {
        let entry = entry.map_err(|e| PackError::File {
            path: root.to_path_buf(),
            message: e.to_string(),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let relative = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if let Some((include, exclude)) = filter
            && (!include.is_match(&relative) || exclude.is_match(&relative))
        {
            continue;
        }
        entries.push(PackageEntry {
            package_path: format!("{prefix}/{relative}"),
            source: entry.path().to_path_buf(),
        });
    }
    Ok(entries)
}

/// Copies a `deploy.conf` without keys that point to the publisher's computer.
/// Returns the removed keys.
fn sanitize_deploy_conf(
    source: &Path,
    target: &Path,
    extra_server_line: Option<&str>,
) -> Result<Vec<String>, PackError> {
    let text = fs::read_to_string(source).map_err(|e| PackError::File {
        path: source.to_path_buf(),
        message: e.to_string(),
    })?;
    let mut removed = Vec::new();
    let mut output = String::new();
    let mut in_server_block = true;
    let mut extra = extra_server_line;
    for line in text.lines() {
        if line.contains(" -> ") {
            in_server_block = false;
            if let Some(extra_line) = extra.take() {
                output.push_str(extra_line);
                output.push_str("\n\n");
            }
        }
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if in_server_block
            && let Some(key) = key
            && MACHINE_SPECIFIC_KEYS.contains(&key)
        {
            removed.push(key.to_owned());
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if let Some(extra_line) = extra {
        output.push_str(extra_line);
        output.push('\n');
    }
    fs::write(target, output).map_err(|e| PackError::File {
        path: target.to_path_buf(),
        message: e.to_string(),
    })?;
    Ok(removed)
}

/// Reads the server's SSH host keys with `ssh-keyscan`. `Ok(None)` for FTPS (no SSH).
fn scan_host_key(deploy_conf: &Path) -> Result<Option<String>, String> {
    let config = DeployConfig::load(deploy_conf).map_err(|e| e.to_string())?;
    if config.server.protocol != Protocol::Sftp {
        return Ok(None);
    }
    let output = Command::new("ssh-keyscan")
        .args(["-T", "10", "-t", "ed25519,ecdsa,rsa", "-p"])
        .arg(config.server.port.to_string())
        .arg(&config.server.host)
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("ssh-keyscan: {e}"))?;
    let keys = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .fold(String::new(), |mut keys, line| {
            keys.push_str(line);
            keys.push('\n');
            keys
        });
    if keys.is_empty() {
        Err(format!("{} nicht erreichbar", config.server.host))
    } else {
        Ok(Some(keys))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wm_core::archive::verify_package;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn project(dir: &Path) -> PathBuf {
        let project = dir.join("project");
        write(
            &project.join(PROJECT_CONFIG),
            r#"{"name":"beispiel","version":"1.0.0","deploy_conf":"deploy.conf",
                "include":["src/**","package.json"],"exclude":["src/secret-notes/**"]}"#,
        );
        write(&project.join("dist/index.html"), "<h1>Hallo</h1>");
        write(&project.join("src/pages/index.astro"), "---\n---");
        write(&project.join("src/secret-notes/todo.md"), "intern");
        write(&project.join("package.json"), "{}");
        write(&project.join("node_modules/x/index.js"), "");
        write(
            &project.join("deploy.conf"),
            "host = h\nuser = u\nkey_file = /Users/me/.ssh/id_rsa\ndist -> /site\n",
        );
        project
    }

    fn key(dir: &Path) -> PathBuf {
        let path = dir.join("signing.key");
        signing::save_signing_key(&signing::generate_signing_key().unwrap(), &path).unwrap();
        path
    }

    #[test]
    fn packs_positive_list_and_sanitizes_deploy_conf() {
        let dir = tempfile::tempdir().unwrap();
        let project = project(dir.path());
        let output = dir.path().join("out.tsweb");
        let report = pack(&PackOptions {
            project,
            key: key(dir.path()),
            output: Some(output.clone()),
            release: None,
            skip_build: true,
            credentials: None,
            package_password: None,
            host_key: false,
        })
        .unwrap();
        assert_eq!(report.version, "1.0.0");
        assert_eq!(report.warnings.len(), 1);

        let files: Vec<String> = verify_package(&output)
            .unwrap()
            .manifest
            .files
            .into_keys()
            .collect();
        assert_eq!(
            files,
            vec![
                "deploy.conf",
                "dist/index.html",
                "project/package.json",
                "project/src/pages/index.astro"
            ]
        );
    }

    #[test]
    fn secrets_abort_packing() {
        let dir = tempfile::tempdir().unwrap();
        let project = project(dir.path());
        write(&project.join("dist/.env"), "SMTP_PASSWORD=x");
        let output = dir.path().join("out.tsweb");
        let result = pack(&PackOptions {
            project,
            key: key(dir.path()),
            output: Some(output.clone()),
            release: None,
            skip_build: true,
            credentials: None,
            package_password: None,
            host_key: false,
        });
        assert!(matches!(
            result,
            Err(PackError::Core(wm_core::CoreError::SecretsFound(_)))
        ));
        assert!(!output.exists());
    }

    #[test]
    fn ships_encrypted_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let project = project(dir.path());
        let output = dir.path().join("out.tsweb");
        let credentials = Credentials {
            password: Some("webspace-geheim".into()),
            private_key: None,
        };
        pack(&PackOptions {
            project,
            key: key(dir.path()),
            output: Some(output.clone()),
            release: None,
            skip_build: true,
            credentials: Some(credentials.clone()),
            package_password: Some("paketpasswort".into()),
            host_key: false,
        })
        .unwrap();
        let target = dir.path().join("open");
        let verified = wm_core::archive::extract_package(&output, &target).unwrap();
        assert!(verified.manifest.has_credentials);
        let content = fs::read_to_string(target.join(CREDENTIALS_FILE)).unwrap();
        assert!(!content.contains("webspace-geheim"));
        assert_eq!(
            credentials::decrypt(&content, "paketpasswort").unwrap(),
            credentials
        );
    }

    #[test]
    fn extra_server_line_goes_before_the_mappings() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("deploy.conf");
        let target = dir.path().join("out.conf");
        write(
            &source,
            "host = h\nuser = u\nkey_file = /k\n\ndist -> /site\n",
        );
        sanitize_deploy_conf(&source, &target, Some("known_hosts_file = known_hosts")).unwrap();
        let text = fs::read_to_string(&target).unwrap();
        let config = DeployConfig::parse(&text, dir.path()).unwrap();
        assert_eq!(
            config.server.known_hosts_file,
            Some(dir.path().join("known_hosts"))
        );
        assert!(config.server.key_file.is_none());
        assert_eq!(config.mappings.len(), 1);
    }

    #[test]
    fn rejects_unsafe_project_name() {
        let dir = tempfile::tempdir().unwrap();
        let project = project(dir.path());
        write(
            &project.join(PROJECT_CONFIG),
            r#"{"name":"../x","version":"1"}"#,
        );
        assert!(matches!(load_config(&project), Err(PackError::Config(_))));
    }
}

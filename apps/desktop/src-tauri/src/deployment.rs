//! Upload settings in effect and the upload itself.
//!
//! Error messages starting with `code:` are translated by the window.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use teleschirm_deploy::{
    Connection, Credential, DeployConfig, Mapping, MappingMode, Protocol, Runner, VersionStamp,
};
use wm_core::projects::{ProjectSettings, read_manifest_copy};

use crate::{keychain, tools};

/// Most paths of a change list sent to the window.
const MAX_LISTED_CHANGES: usize = 1000;

/// Package `deploy.conf` with the customer's settings applied. The second value says whether
/// the host key recorded in the package applies (same SFTP server as in the package).
///
/// # Errors
/// Fails if the `deploy.conf` or the expert mappings are invalid.
pub fn effective_config(
    folder: &Path,
    settings: &ProjectSettings,
) -> Result<(DeployConfig, bool), String> {
    let mut config = DeployConfig::load(&folder.join("deploy.conf")).map_err(|e| e.to_string())?;
    // Mapping sources are relative to the package root, where `dist/` lives.
    config.base_dir = folder.to_path_buf();
    let packaged_server = (config.server.host.clone(), config.server.port);

    if let Some(text) = &settings.mappings {
        config.mappings = parse_mappings(text)?;
    }
    if let Some(root) = &settings.remote_root {
        config = config.with_remote_root(root);
    }
    if let Some(protocol) = &settings.protocol {
        config.server.protocol = match protocol.as_str() {
            "sftp" => Protocol::Sftp,
            "ftps" => Protocol::Ftps,
            other => return Err(format!("unknown protocol {other:?}")),
        };
    }
    if let Some(host) = &settings.host {
        config.server.host.clone_from(host);
    }
    if let Some(user) = &settings.user {
        config.server.user.clone_from(user);
    }
    if let Some(port) = settings.port {
        config.server.port = port;
    }

    let pinned = config.server.protocol == Protocol::Sftp
        && config
            .server
            .known_hosts_file
            .as_ref()
            .is_some_and(|file| file.is_file())
        && (config.server.host.clone(), config.server.port) == packaged_server;
    if !pinned {
        // Recorded keys belong to the packaged server only.
        config.server.known_hosts_file = None;
    }
    Ok((config, pinned))
}

/// Parses expert mapping lines (`<local> -> <remote> | <options>`).
///
/// # Errors
/// Fails for invalid lines or if no mapping remains.
pub fn parse_mappings(text: &str) -> Result<Vec<Mapping>, String> {
    let config = DeployConfig::parse(&format!("host = x\nuser = x\n{text}"), Path::new("."))
        .map_err(|e| e.to_string())?;
    if config.mappings.is_empty() {
        return Err("code:no-mappings".into());
    }
    Ok(config.mappings)
}

/// Where the window wants to upload to.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UploadTarget {
    /// The webspace. Without a password the stored credentials are used.
    Server { password: Option<String> },
    /// A local folder standing in for the server.
    LocalFolder { folder: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadRequest {
    pub target: UploadTarget,
    pub dry_run: bool,
    /// Upload even if the server has a newer version (restore).
    pub allow_older: bool,
    /// Upload to an SFTP server whose identity is not recorded in the package.
    pub accept_unverified_host: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub dry_run: bool,
    pub done: usize,
    pub total: usize,
    pub skipped_seed: usize,
    pub uploaded_count: usize,
    pub deleted_count: usize,
    pub uploaded: Vec<String>,
    pub deleted: Vec<String>,
    pub website: Option<String>,
}

/// Removes a temporary key file when the upload ends.
struct TemporaryKey(PathBuf);

impl Drop for TemporaryKey {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_temporary_key(folder: &Path, key: &str) -> Result<TemporaryKey, String> {
    let dir = folder.join(".runtime");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("upload-key");
    fs::write(&path, key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(TemporaryKey(path))
}

/// Uploads the version in `folder`. Every rclone line goes to `emit`.
///
/// # Errors
/// Fails with a `code:` message for missing credentials, an unverified server or a newer
/// version on the server, otherwise with rclone's message.
pub fn run(
    folder: &Path,
    project: &str,
    settings: &ProjectSettings,
    request: UploadRequest,
    emit: &mut dyn FnMut(&str),
) -> Result<UploadResult, String> {
    let manifest = read_manifest_copy(folder).map_err(|e| e.to_string())?;
    let (mut config, pinned) = effective_config(folder, settings)?;
    let mut _key_guard = None;

    let connection = match request.target {
        UploadTarget::LocalFolder { folder } => Connection::LocalFolder(PathBuf::from(folder)),
        UploadTarget::Server { password } => {
            if config.server.protocol == Protocol::Sftp
                && !pinned
                && !request.accept_unverified_host
            {
                return Err(format!("code:unverified-host {}", config.server.host));
            }
            if let Some(password) = password.filter(|text| !text.is_empty()) {
                Connection::Server(Credential::Password(password))
            } else {
                let stored = keychain::load(project)?.ok_or("code:no-credentials")?;
                if let Some(key) = &stored.private_key {
                    let guard = write_temporary_key(folder, key)?;
                    config.server.key_file = Some(guard.0.clone());
                    _key_guard = Some(guard);
                    Connection::Server(Credential::KeyFile)
                } else if let Some(password) = stored.password {
                    Connection::Server(Credential::Password(password))
                } else {
                    return Err("code:no-credentials".into());
                }
            }
        }
    };

    let rclone = tools::rclone();
    emit(&format!(
        "rclone ({}): {}",
        rclone.source.label(),
        rclone.program.display()
    ));
    let runner = Runner::new(rclone.program, request.dry_run);
    if let Some(on_server) = runner
        .read_server_version(&config, &connection)
        .map_err(|e| e.to_string())?
    {
        emit(&format!(
            "on the server: {} {} ({})",
            on_server.name, on_server.version, on_server.created
        ));
        if on_server.created > manifest.created && !request.allow_older {
            return Err(format!(
                "code:newer-on-server {} {}",
                on_server.version, on_server.created
            ));
        }
    }

    let stamp = VersionStamp {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        created: manifest.created.clone(),
    };
    let summary = runner
        .run(&config, MappingMode::Sync, &connection, Some(&stamp), emit)
        .map_err(|e| e.to_string())?;
    let mut uploaded = summary.uploaded;
    let mut deleted = summary.deleted;
    let (uploaded_count, deleted_count) = (uploaded.len(), deleted.len());
    uploaded.truncate(MAX_LISTED_CHANGES);
    deleted.truncate(MAX_LISTED_CHANGES);
    Ok(UploadResult {
        dry_run: summary.dry_run,
        done: summary.done,
        total: summary.total,
        skipped_seed: summary.skipped_seed,
        uploaded_count,
        deleted_count,
        uploaded,
        deleted,
        website: settings.domain.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(dir: &Path, with_known_hosts: bool) {
        let mut conf = String::from("host = example.com\nuser = web1\n");
        if with_known_hosts {
            conf.push_str("known_hosts_file = known_hosts\n");
            fs::write(dir.join("known_hosts"), "example.com ssh-ed25519 AAAA\n").unwrap();
        }
        conf.push_str("dist/backend/data -> /site/backend/data | mode=seed\ndist -> /site\n");
        fs::write(dir.join("deploy.conf"), conf).unwrap();
    }

    #[test]
    fn settings_override_the_package() {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), true);
        let settings = ProjectSettings {
            user: Some("kunde".into()),
            remote_root: Some("/httpdocs".into()),
            ..ProjectSettings::default()
        };
        let (config, pinned) = effective_config(dir.path(), &settings).unwrap();
        assert!(pinned);
        assert_eq!(config.server.user, "kunde");
        assert_eq!(config.mappings[1].target, "/httpdocs");
        assert_eq!(config.base_dir, dir.path());
    }

    #[test]
    fn other_host_drops_the_recorded_key() {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), true);
        let settings = ProjectSettings {
            host: Some("other.example.com".into()),
            ..ProjectSettings::default()
        };
        let (config, pinned) = effective_config(dir.path(), &settings).unwrap();
        assert!(!pinned);
        assert!(config.server.known_hosts_file.is_none());
    }

    #[test]
    fn package_without_key_is_not_pinned() {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), false);
        let (_, pinned) = effective_config(dir.path(), &ProjectSettings::default()).unwrap();
        assert!(!pinned);
    }

    #[test]
    fn expert_mappings_replace_the_package_ones() {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), false);
        let settings = ProjectSettings {
            mappings: Some("dist -> /www | protect=data/**\n".into()),
            ..ProjectSettings::default()
        };
        let (config, _) = effective_config(dir.path(), &settings).unwrap();
        assert_eq!(config.mappings.len(), 1);
        assert_eq!(config.mappings[0].protect, vec!["data/**"]);
        assert!(parse_mappings("# nur Kommentar\n").is_err());
        assert!(parse_mappings("dist -> /x | delete=all").is_err());
    }

    #[test]
    fn uploads_into_a_local_folder_and_refuses_older_versions() {
        if std::process::Command::new("rclone")
            .arg("version")
            .output()
            .is_err()
        {
            eprintln!("rclone not found – test skipped");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("version");
        let server = dir.path().join("server");
        fs::create_dir_all(folder.join("dist/backend/data")).unwrap();
        fs::write(folder.join("dist/index.html"), "neu").unwrap();
        package(&folder, false);
        let manifest = |created: &str| {
            format!(
                r#"{{"format_version":1,"name":"cafe","version":"1","created":"{created}","files":{{}}}}"#
            )
        };
        fs::write(
            folder.join(wm_core::projects::MANIFEST_COPY),
            manifest("2026-11-27T08:00:00Z"),
        )
        .unwrap();
        let request = |allow_older| UploadRequest {
            target: UploadTarget::LocalFolder {
                folder: server.to_string_lossy().into_owned(),
            },
            dry_run: false,
            allow_older,
            accept_unverified_host: false,
        };
        let mut lines = Vec::new();
        let result = run(
            &folder,
            "cafe",
            &ProjectSettings::default(),
            request(false),
            &mut |line| {
                lines.push(line.to_owned());
            },
        )
        .unwrap();
        assert!(
            result.uploaded.contains(&"/site/index.html".to_owned()),
            "{:?}",
            result.uploaded
        );
        assert!(!lines.is_empty(), "progress lines are emitted");
        assert!(server.join("site/.website-manager.json").is_file());

        // An older package must not silently replace the newer one on the server.
        fs::write(
            folder.join(wm_core::projects::MANIFEST_COPY),
            manifest("2026-10-01T08:00:00Z"),
        )
        .unwrap();
        let error = run(
            &folder,
            "cafe",
            &ProjectSettings::default(),
            request(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(error.starts_with("code:newer-on-server"), "{error}");
        assert!(
            run(
                &folder,
                "cafe",
                &ProjectSettings::default(),
                request(true),
                &mut |_| {}
            )
            .is_ok()
        );
    }

    #[test]
    fn unverified_server_needs_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), false);
        fs::write(
            dir.path().join(wm_core::projects::MANIFEST_COPY),
            r#"{"format_version":1,"name":"cafe","version":"1","created":"2026-11-27T08:00:00Z","files":{}}"#,
        )
        .unwrap();
        let request = UploadRequest {
            target: UploadTarget::Server {
                password: Some("x".into()),
            },
            dry_run: true,
            allow_older: false,
            accept_unverified_host: false,
        };
        let error = run(
            dir.path(),
            "cafe",
            &ProjectSettings::default(),
            request,
            &mut |_| {},
        )
        .unwrap_err();
        assert!(error.starts_with("code:unverified-host"), "{error}");
    }
}

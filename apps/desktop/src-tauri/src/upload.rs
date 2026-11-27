//! Upload of an opened package according to its `deploy.conf`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use teleschirm_deploy::{Connection, Credential, DeployConfig, MappingMode, Runner, VersionStamp};

use crate::packages::load_manifest;
use crate::tools;

/// Where the window wants to upload to.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UploadTarget {
    /// The server from the `deploy.conf`, logging in with a password.
    Server { password: String },
    /// A local folder standing in for the server.
    LocalFolder { folder: String },
}

/// Result shown in the window.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub dry_run: bool,
    pub done: usize,
    pub total: usize,
    pub skipped_seed: usize,
    pub log: Vec<String>,
}

/// Uploads the package in `folder`. Refuses to overwrite a newer version on the server
/// unless `allow_older` is set.
///
/// # Errors
/// Fails if the configuration is missing, the server has a newer version or rclone fails.
pub fn run(
    folder: &Path,
    target: UploadTarget,
    dry_run: bool,
    allow_older: bool,
) -> Result<UploadResult, String> {
    let manifest = load_manifest(folder)?;
    let mut config = DeployConfig::load(&folder.join("deploy.conf")).map_err(|e| e.to_string())?;
    // Mapping sources are relative to the package root, where `dist/` lives.
    config.base_dir = folder.to_path_buf();
    let connection = match target {
        UploadTarget::Server { password } => Connection::Server(Credential::Password(password)),
        UploadTarget::LocalFolder { folder } => Connection::LocalFolder(PathBuf::from(folder)),
    };

    let rclone = tools::rclone();
    let mut log = vec![format!(
        "rclone ({}): {}",
        rclone.source,
        rclone.program.display()
    )];
    let runner = Runner::new(rclone.program, dry_run);

    if let Some(on_server) = runner
        .read_server_version(&config, &connection)
        .map_err(|e| e.to_string())?
    {
        log.push(format!(
            "on the server: {} {} ({})",
            on_server.name, on_server.version, on_server.created
        ));
        if on_server.created > manifest.created && !allow_older {
            return Err(format!(
                "The server already has a newer version ({} from {}). This package is {} from {}.",
                on_server.version, on_server.created, manifest.version, manifest.created
            ));
        }
    }

    let stamp = VersionStamp {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        created: manifest.created.clone(),
    };
    let summary = runner
        .run(
            &config,
            MappingMode::Sync,
            &connection,
            Some(&stamp),
            &mut |line| {
                log.push(line.to_owned());
            },
        )
        .map_err(|e| format!("{e}\n{}", log.join("\n")))?;
    Ok(UploadResult {
        dry_run: summary.dry_run,
        done: summary.done,
        total: summary.total,
        skipped_seed: summary.skipped_seed,
        log,
    })
}

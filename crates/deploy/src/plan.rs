//! Turns a [`DeployConfig`] into rclone invocations.

use std::path::PathBuf;

use crate::config::{DeployConfig, MappingMode, Protocol};
use crate::{DeployError, Mapping, VERSION_FILE};

/// Files that are never uploaded (same list as `deploy.sh`).
const DEFAULT_EXCLUDES: &[&str] = &[
    "node_modules/**",
    ".git/**",
    ".DS_Store",
    ".venv/**",
    ".astro/**",
    "Thumbs.db",
];

/// How to log in to the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    /// SSH key from the `deploy.conf`.
    KeyFile,
    /// Password; handed to rclone through an environment variable, never as an argument.
    Password(String),
}

/// Where the upload goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    /// The server from the `deploy.conf`.
    Server(Credential),
    /// A local folder that stands in for the server (tests, previews of the result).
    LocalFolder(PathBuf),
}

/// One rclone call for one mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub mapping: Mapping,
    pub mode: MappingMode,
    /// Local source folder.
    pub source: PathBuf,
    /// rclone remote path, e.g. `:sftp:/site` or a local path.
    pub remote: String,
    /// Arguments after the rclone sub-command, source and remote.
    pub filter_args: Vec<String>,
}

impl Step {
    /// Full argument list for rclone (without connection flags).
    #[must_use]
    pub fn rclone_args(&self, dry_run: bool) -> Vec<String> {
        let command = match self.mode {
            MappingMode::Sync => "sync",
            MappingMode::Copy | MappingMode::Seed => "copy",
        };
        let mut args = vec![
            command.to_owned(),
            self.source.to_string_lossy().into_owned(),
            self.remote.clone(),
        ];
        if self.mode == MappingMode::Seed {
            args.push("--ignore-existing".into());
        }
        if dry_run {
            args.push("--dry-run".into());
        }
        args.extend(self.filter_args.iter().cloned());
        args
    }
}

/// Builds the upload steps. `run_mode` is used for mappings without their own `mode`.
///
/// # Errors
/// Fails if a local source folder is missing.
pub fn build_plan(
    config: &DeployConfig,
    run_mode: MappingMode,
    connection: &Connection,
) -> Result<Vec<Step>, DeployError> {
    let site_root = site_root(config);
    config
        .mappings
        .iter()
        .map(|mapping| {
            let source = config.base_dir.join(&mapping.source);
            if !source.is_dir() {
                return Err(DeployError::MissingSource {
                    source_dir: mapping.source.clone(),
                    path: source,
                });
            }
            let mut filter_args = Vec::new();
            for pattern in DEFAULT_EXCLUDES
                .iter()
                .copied()
                .map(str::to_owned)
                .chain(mapping.protect.iter().cloned())
            {
                filter_args.push("--exclude".into());
                filter_args.push(pattern);
            }
            if site_root.as_deref() == Some(mapping.target.as_str()) {
                // The version file is written last and must survive `sync`.
                filter_args.push("--exclude".into());
                filter_args.push(format!("/{VERSION_FILE}"));
            }
            Ok(Step {
                mapping: mapping.clone(),
                mode: mapping.mode.unwrap_or(run_mode),
                source,
                remote: remote_path(config, connection, &mapping.target),
                filter_args,
            })
        })
        .collect()
}

/// The shortest target of all mappings; the version file lives there.
#[must_use]
pub fn site_root(config: &DeployConfig) -> Option<String> {
    config
        .mappings
        .iter()
        .map(|mapping| mapping.target.clone())
        .min_by_key(String::len)
}

/// rclone path for `target` on the given connection.
#[must_use]
pub fn remote_path(config: &DeployConfig, connection: &Connection, target: &str) -> String {
    match connection {
        Connection::Server(_) => {
            let backend = match config.server.protocol {
                Protocol::Sftp => "sftp",
                Protocol::Ftps => "ftp",
            };
            format!(":{backend}:{target}")
        }
        Connection::LocalFolder(root) => root
            .join(target.trim_start_matches('/'))
            .to_string_lossy()
            .into_owned(),
    }
}

/// Connection flags and environment variables for rclone. The password must already be
/// obscured with `rclone obscure`.
#[must_use]
pub fn connection_settings(
    config: &DeployConfig,
    connection: &Connection,
    obscured_password: Option<&str>,
) -> (Vec<String>, Vec<(String, String)>) {
    let Connection::Server(credential) = connection else {
        return (Vec::new(), Vec::new());
    };
    let server = &config.server;
    let mut flags = Vec::new();
    let mut env = Vec::new();
    match server.protocol {
        Protocol::Sftp => {
            flags.extend([
                format!("--sftp-host={}", server.host),
                format!("--sftp-user={}", server.user),
                format!("--sftp-port={}", server.port),
                // The chroot jails of shared hosting have no shell and no checksum tools.
                "--sftp-shell-type=none".into(),
                "--sftp-md5sum-command=none".into(),
                "--sftp-sha1sum-command=none".into(),
            ]);
            if *credential == Credential::KeyFile
                && let Some(key) = &server.key_file
            {
                flags.push(format!("--sftp-key-file={}", key.display()));
            }
            if let Some(known_hosts) = &server.known_hosts_file {
                flags.push(format!("--sftp-known-hosts-file={}", known_hosts.display()));
            }
            if let Some(password) = obscured_password {
                env.push(("RCLONE_SFTP_PASS".into(), password.to_owned()));
            }
        }
        Protocol::Ftps => {
            flags.extend([
                format!("--ftp-host={}", server.host),
                format!("--ftp-user={}", server.user),
                format!("--ftp-port={}", server.port),
                "--ftp-explicit-tls".into(),
            ]);
            if let Some(password) = obscured_password {
                env.push(("RCLONE_FTP_PASS".into(), password.to_owned()));
            }
        }
    }
    (flags, env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn config(base: &Path) -> DeployConfig {
        let text = "\
host = example.com
user = web1
key_file = /keys/id
dist/backend/data -> /site/backend/data | mode=seed
dist -> /site | protect=backend/data/*.json
";
        let mut config = DeployConfig::parse(text, base).unwrap();
        config.base_dir = base.to_path_buf();
        config
    }

    #[test]
    fn plan_for_server() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("dist/backend/data")).unwrap();
        let config = config(dir.path());
        let steps = build_plan(
            &config,
            MappingMode::Sync,
            &Connection::Server(Credential::KeyFile),
        )
        .unwrap();

        assert_eq!(steps[0].mode, MappingMode::Seed);
        assert_eq!(steps[0].remote, ":sftp:/site/backend/data");
        let seed_args = steps[0].rclone_args(false);
        assert_eq!(seed_args[0], "copy");
        assert!(seed_args.contains(&"--ignore-existing".to_owned()));

        assert_eq!(steps[1].mode, MappingMode::Sync);
        let sync_args = steps[1].rclone_args(true);
        assert_eq!(sync_args[0], "sync");
        assert!(sync_args.contains(&"--dry-run".to_owned()));
        assert!(
            sync_args
                .windows(2)
                .any(|pair| pair == ["--exclude", "backend/data/*.json"])
        );
        assert!(
            sync_args
                .windows(2)
                .any(|pair| pair == ["--exclude", "/.website-manager.json"])
        );
        assert!(!seed_args.contains(&"/.website-manager.json".to_owned()));
    }

    #[test]
    fn missing_source_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        assert!(matches!(
            build_plan(
                &config,
                MappingMode::Sync,
                &Connection::Server(Credential::KeyFile)
            ),
            Err(DeployError::MissingSource { .. })
        ));
    }

    #[test]
    fn password_goes_to_environment_not_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let connection = Connection::Server(Credential::Password("geheim".into()));
        let (flags, env) = connection_settings(&config, &connection, Some("obscured"));
        assert!(
            flags
                .iter()
                .all(|flag| !flag.contains("obscured") && !flag.contains("geheim"))
        );
        assert!(!flags.iter().any(|flag| flag.starts_with("--sftp-key-file")));
        assert_eq!(
            env,
            vec![("RCLONE_SFTP_PASS".to_owned(), "obscured".to_owned())]
        );
    }

    #[test]
    fn local_folder_paths() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let connection = Connection::LocalFolder(PathBuf::from("/tmp/server"));
        assert_eq!(
            remote_path(&config, &connection, "/site"),
            "/tmp/server/site"
        );
    }
}

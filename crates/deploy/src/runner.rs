//! Runs the planned steps with rclone.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::changes::{ChangeKind, parse_line};
use crate::config::{DeployConfig, MappingMode};
use crate::plan::{
    Connection, Credential, build_plan, connection_settings, remote_path, site_root,
};
use crate::{DeployError, VERSION_FILE};

/// Content of the version file on the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionStamp {
    pub name: String,
    pub version: String,
    /// Creation time of the uploaded package (RFC 3339); compared to detect older packages.
    pub created: String,
}

/// Outcome of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    pub total: usize,
    pub done: usize,
    pub skipped_seed: usize,
    pub dry_run: bool,
    /// Server paths that were (or in a dry run would be) uploaded.
    pub uploaded: Vec<String>,
    /// Server paths that were (or in a dry run would be) deleted.
    pub deleted: Vec<String>,
}

/// Executes uploads with a given rclone program.
pub struct Runner {
    rclone: PathBuf,
    dry_run: bool,
}

impl Runner {
    #[must_use]
    pub fn new(rclone: PathBuf, dry_run: bool) -> Self {
        Self { rclone, dry_run }
    }

    /// Reads the version file from the server; `None` if there is none yet.
    ///
    /// # Errors
    /// Fails if rclone cannot be started or the file exists but is invalid.
    pub fn read_server_version(
        &self,
        config: &DeployConfig,
        connection: &Connection,
    ) -> Result<Option<VersionStamp>, DeployError> {
        let Some(root) = site_root(config) else {
            return Ok(None);
        };
        let remote = format!("{}/{VERSION_FILE}", remote_path(config, connection, &root));
        let output = self
            .command(config, connection, &["cat".into(), remote])?
            .stderr(Stdio::null())
            .output()
            .map_err(|source| self.start_error(source))?;
        if !output.status.success() || output.stdout.is_empty() {
            return Ok(None);
        }
        serde_json::from_slice(&output.stdout)
            .map(Some)
            .map_err(|e| DeployError::VersionFile(e.to_string()))
    }

    /// Uploads all mappings, then writes `stamp` as version file (not in a dry run).
    /// Every output line of rclone is passed to `log`.
    ///
    /// # Errors
    /// Fails if planning fails, rclone cannot be started or a mapping fails.
    pub fn run(
        &self,
        config: &DeployConfig,
        run_mode: MappingMode,
        connection: &Connection,
        stamp: Option<&VersionStamp>,
        log: &mut dyn FnMut(&str),
    ) -> Result<RunSummary, DeployError> {
        let steps = build_plan(config, run_mode, connection)?;
        let mut summary = RunSummary {
            total: steps.len(),
            done: 0,
            skipped_seed: 0,
            dry_run: self.dry_run,
            uploaded: Vec::new(),
            deleted: Vec::new(),
        };
        let mut failed = 0;

        for step in &steps {
            log(&format!(
                "==> [{:?}] {} -> {}",
                step.mode, step.mapping.source, step.mapping.target
            ));
            if step.mode == MappingMode::Seed && self.has_files(config, connection, &step.remote)? {
                log("    target already has content – skipped");
                summary.skipped_seed += 1;
                summary.done += 1;
                continue;
            }
            let mut args = step.rclone_args(self.dry_run);
            args.extend(
                [
                    "--transfers",
                    "4",
                    "--checkers",
                    "8",
                    "--retries",
                    "1",
                    "--verbose",
                ]
                .map(str::to_owned),
            );
            let target = step.mapping.target.trim_end_matches('/').to_owned();
            let mut on_line = |line: &str| {
                if let Some((kind, path)) = parse_line(line) {
                    let full = format!("{target}/{path}");
                    match kind {
                        ChangeKind::Upload => summary.uploaded.push(full),
                        ChangeKind::Delete => summary.deleted.push(full),
                    }
                }
                log(line);
            };
            if self.stream(config, connection, &args, &mut on_line)? {
                summary.done += 1;
                log("    OK");
            } else {
                failed += 1;
                log("    FAILED");
            }
        }

        if failed > 0 {
            return Err(DeployError::MappingsFailed {
                failed,
                total: summary.total,
            });
        }
        if !self.dry_run
            && let (Some(stamp), Some(root)) = (stamp, site_root(config))
        {
            self.write_version_file(config, connection, &root, stamp)?;
            log(&format!("==> version file {VERSION_FILE} written"));
        }
        Ok(summary)
    }

    fn has_files(
        &self,
        config: &DeployConfig,
        connection: &Connection,
        remote: &str,
    ) -> Result<bool, DeployError> {
        let args = [
            "lsf".to_owned(),
            remote.to_owned(),
            "-R".into(),
            "--files-only".into(),
            "--exclude".into(),
            ".htaccess".into(),
        ];
        let output = self
            .command(config, connection, &args)?
            .stderr(Stdio::null())
            .output()
            .map_err(|source| self.start_error(source))?;
        Ok(output.status.success() && output.stdout.iter().any(|byte| !byte.is_ascii_whitespace()))
    }

    fn write_version_file(
        &self,
        config: &DeployConfig,
        connection: &Connection,
        root: &str,
        stamp: &VersionStamp,
    ) -> Result<(), DeployError> {
        let remote = format!("{}/{VERSION_FILE}", remote_path(config, connection, root));
        let content = serde_json::to_vec_pretty(stamp)
            .map_err(|e| DeployError::VersionFile(e.to_string()))?;
        let mut child = self
            .command(config, connection, &["rcat".into(), remote])?
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| self.start_error(source))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&content)
                .map_err(|e| DeployError::RcloneFailed(e.to_string()))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|e| DeployError::RcloneFailed(e.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(DeployError::RcloneFailed(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }

    /// Runs rclone and passes stdout and stderr lines to `log`. Returns whether it succeeded.
    fn stream(
        &self,
        config: &DeployConfig,
        connection: &Connection,
        args: &[String],
        log: &mut dyn FnMut(&str),
    ) -> Result<bool, DeployError> {
        let mut child = self
            .command(config, connection, args)?
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| self.start_error(source))?;
        // rclone writes its progress and --verbose output to stderr.
        if let Some(stderr) = child.stderr.take() {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                log(&format!("    {line}"));
            }
        }
        let status = child
            .wait()
            .map_err(|e| DeployError::RcloneFailed(e.to_string()))?;
        Ok(status.success())
    }

    fn command(
        &self,
        config: &DeployConfig,
        connection: &Connection,
        args: &[String],
    ) -> Result<Command, DeployError> {
        let obscured = match connection {
            Connection::Server(Credential::Password(password)) => Some(self.obscure(password)?),
            _ => None,
        };
        let (flags, env) = connection_settings(config, connection, obscured.as_deref());
        let mut command = Command::new(&self.rclone);
        command.args(args).args(flags).envs(env);
        // Keep the user's rclone.conf out of the run; everything comes from deploy.conf.
        command.env("RCLONE_CONFIG", null_device());
        Ok(command)
    }

    fn obscure(&self, password: &str) -> Result<String, DeployError> {
        let mut child = Command::new(&self.rclone)
            .args(["obscure", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| self.start_error(source))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(password.as_bytes())
                .map_err(|e| DeployError::RcloneFailed(e.to_string()))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|e| DeployError::RcloneFailed(e.to_string()))?;
        if !output.status.success() {
            return Err(DeployError::RcloneFailed("rclone obscure failed".into()));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn start_error(&self, source: std::io::Error) -> DeployError {
        DeployError::RcloneStart {
            program: self.rclone.clone(),
            source,
        }
    }
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

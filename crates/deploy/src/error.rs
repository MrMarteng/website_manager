//! The error type of `teleschirm-deploy`.

use std::path::PathBuf;

/// Errors while reading a `deploy.conf`, planning or running an upload.
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    /// The `deploy.conf` could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path of the `deploy.conf`.
        path: PathBuf,
        /// Underlying operating system error.
        #[source]
        source: std::io::Error,
    },
    /// A line of the `deploy.conf` is invalid.
    #[error("deploy.conf line {line}: {message}")]
    Syntax {
        /// 1-based line number.
        line: usize,
        /// What is wrong with it.
        message: String,
    },
    /// A required setting (such as the host) is missing.
    #[error("deploy.conf is incomplete: {0}")]
    Incomplete(String),
    /// The local folder of a mapping does not exist.
    #[error("local folder of mapping {source_dir:?} is missing: {path}")]
    MissingSource {
        /// Source as written in the mapping.
        source_dir: String,
        /// Resolved path that is missing.
        path: PathBuf,
    },
    /// The rclone program could not be started.
    #[error("rclone could not be started ({program}): {source}")]
    RcloneStart {
        /// Program that was started.
        program: PathBuf,
        /// Underlying operating system error.
        #[source]
        source: std::io::Error,
    },
    /// An rclone helper call (obscure, write the version file) failed.
    #[error("rclone failed: {0}")]
    RcloneFailed(String),
    /// At least one mapping could not be uploaded; details are in the log.
    #[error("{failed} of {total} mapping(s) failed")]
    MappingsFailed {
        /// Number of failed mappings.
        failed: usize,
        /// Number of mappings in the run.
        total: usize,
    },
    /// The run was cancelled; mappings may be partly uploaded, no version file was written.
    #[error("cancelled")]
    Cancelled,
    /// The version file on the server exists but is not valid.
    #[error("version file on the server is unreadable: {0}")]
    VersionFile(String),
}
